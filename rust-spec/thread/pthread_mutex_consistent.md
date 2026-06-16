# pthread_mutex_consistent — Rust 接口归约

## 原始 C 接口
```c
int pthread_mutex_consistent(pthread_mutex_t *m);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> core::ffi::c_int;
```

---

## 互斥锁位域布局（回顾）

`_m_lock` 字段：

| 位 | 掩码 | 含义 |
|----|------|------|
| 0–29 | `0x3fffffff` | 持有者 tid |
| 30 | `0x40000000` | EOWNERDEAD 标记 |
| 31 | `0x80000000` | 等待者标记 |

`_m_type` 字段位 2 (`& 4`) 为 `PTHREAD_MUTEX_ROBUST` 标志。

---

## 意图
当健壮互斥锁被获取但检测到前一持有者已终止（`EOWNERDEAD`），调用者在修复受保护的共享数据后，调用此函数将互斥锁标记为一致状态。调用者必须是当前互斥锁的持有者。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向一个已初始化的 `pthread_mutex_t`
- 互斥锁必须是 ROBUST 类型（`_m_type & 4` 非零）
- 互斥锁处于 EOWNERDEAD 状态（`_m_lock` 位 30 已设置）
- 互斥锁当前被某线程持有（`own = _m_lock & 0x3fffffff` 非零）
- 调用者必须已通过 `pthread_mutex_lock` 获取了此互斥锁

## 后置条件
- Case 1 成功 — 互斥锁为 ROBUST、处于 EOWNERDEAD、被当前线程持有：
  - `_m_lock` 的位 30（`0x40000000`）被原子清除
  - 互斥锁恢复为一致状态，可作为普通锁继续使用
  - 返回值为 `0`
- Case 2 互斥锁非 ROBUST（`!(type & 4)`）、或无持有者（`own == 0`）、或未处于 EOWNERDEAD（`!(old & 0x40000000)`）：
  - 互斥锁状态不变
  - 返回值为 `EINVAL`
- Case 3 调用者非当前持有者（`own != current_tid`）：
  - 互斥锁状态不变
  - 返回值为 `EPERM`

## 不变量
- 仅清除 `_m_lock` 的 EOWNERDEAD 标记位，不影响所有者 tid 和 waiters 标志
- 操作必须是原子的，与并发的等待/锁操作正确同步

## 算法

```rust
extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    // 1. 读取 _m_lock 和 _m_type
    // 2. 校验：ROBUST + 有持有者 + EOWNERDEAD 已设置
    // 3. 校验：当前线程是持有者（通过内部 thread_self() 获取 tid）
    // 4. 原子清除 EOWNERDEAD 标记（位 30）
    // 5. 返回 0
}
```

内部 Rust 实现使用 `core::sync::atomic::AtomicI32` 的 `fetch_and` 进行原子位清除操作，替代 C 的 `a_and` 宏。线程 tid 通过内部 `current_thread()` 函数获取。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
pub(crate) fn mutex_consistent(m: &PthreadMutex) -> Result<(), core::ffi::c_int>;
```

典型的 robust mutex 恢复流程：

```
1. pthread_mutex_lock(&m) → 返回 EOWNERDEAD
2. (修复被保护数据)
3. mutex_consistent(&m) → 返回 Ok(())，标记一致性
4. 正常使用，最终 pthread_mutex_unlock(&m)
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  current_thread() (内部模块)      // 依赖: 获取当前线程控制块/tid
  core::sync::atomic::AtomicI32    // 依赖: 原子 fetch_and 清除 EOWNERDEAD 标记

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_consistent 符号
Internal Interface:
  pub(crate) fn mutex_consistent(m: &PthreadMutex) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
