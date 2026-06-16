# pthread_mutex_trylock — Rust 接口归约

## 原始 C 接口
```c
int __pthread_mutex_trylock(pthread_mutex_t *m);
int pthread_mutex_trylock(pthread_mutex_t *m);  // weak_alias → __pthread_mutex_trylock
```

---

## Rust 外部 ABI 接口

```rust
// musl 中 __pthread_mutex_trylock 是主实现，pthread_mutex_trylock 是其 weak_alias
// rusl 必须同时导出两者
extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
extern "C" fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
```

---

## 互斥锁位域布局

`_m_lock` 字段编码锁状态和所有权：

| 位 | 掩码 | 含义 |
|----|------|------|
| 0–29 | `0x3fffffff` | 持有者 tid（正常），或 `0x3fffffff`=死锁/不可恢复 |
| 30 | `0x40000000` | 死锁标记 (EOWNERDEAD) — robust 互斥锁持有者终止 |
| 31 | `0x80000000` | 等待者标记 — 有线程在等待此锁 |

`EBUSY`(16) 用作 NORMAL 类型互斥锁的"已锁定"标记值。

---

## 意图
以非阻塞方式尝试获取互斥锁。若成功则锁定互斥锁，否则立即返回 EBUSY。NORMAL 类型互斥锁走 CAS 快速路径，其他类型委托给内部所有权跟踪路径进行 robust list 注册和递归/死锁检测。

## 前置条件
- `m` 非空指针（`!m.is_null()`）
- `m` 指向一个已初始化的 `pthread_mutex_t`

## 后置条件
- Case 1 NORMAL 类型且锁空闲，CAS 成功：
  - `_m_lock` 从 `0` 变为 `EBUSY`(16)
  - 返回值为 `0`
- Case 2 NORMAL 类型且锁被占用，CAS 失败：
  - 返回值为 `EBUSY`(16)
- Case 3 其他类型且锁空闲，成功获取：
  - 若为首次使用 process-shared 互斥锁：内部注册 robust_list
  - 若 `old` 非零（之前状态为 EOWNERDEAD）：返回 `EOWNERDEAD`
  - 锁被加入线程的 robust_list 链表
  - 返回值为 `0`
- Case 4 递归重入（RECURSIVE 类型且 own == tid）：
  - `_m_count` 递增（上限 INT_MAX，超过返回 `EAGAIN`）
  - 返回值为 `0`
- Case 5 锁已被拥有且非递归重入：
  - 返回值为 `EBUSY`
- Case 6 不可恢复状态（own == 0x3fffffff）：
  - 返回值为 `ENOTRECOVERABLE`
- Case 7 PI 类型 spurious success：
  - 返回值为 `ENOTRECOVERABLE` 或 `EBUSY`

## 不变量
- `robust_list.pending` 在函数返回前必然被清零
- process-shared 互斥锁的 robust_list 仅在首次使用时惰性注册
- 成功获取后互斥锁被正确链接到线程的 robust list 链表中

## 算法
两阶段策略——快速路径（NORMAL CAS）和所有权跟踪路径：

```rust
extern "C" fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    __pthread_mutex_trylock(m)
}

extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int {
    unsafe {
        let mref = &*m;
        // 阶段 1: NORMAL 类型快速路径
        if (mref.get_type() & 15) == MUTEX_TYPE_NORMAL {
            let result = atomic_cas(&mref.lock(), 0, EBUSY);
            return result & EBUSY;
        }
        // 阶段 2: 其他类型 — 所有权跟踪路径
        trylock_owner_impl(m)
    }
}
```

内部 `trylock_owner_impl` 函数处理：
1. 获取当前线程 tid（`current_thread()`）
2. 递归重入检测（RECURSIVE 类型、`_m_count` 递增）
3. process-shared 互斥锁的 robust_list 惰性注册（`SYS_set_robust_list`）
4. PI spurious success 检测和恢复（`FUTEX_UNLOCK_PI`）
5. CAS 竞态失败处理
6. 成功获取后将互斥锁链接到 robust_list 链表

`__pthread_mutex_trylock_owner`（C 内部符号）对应的逻辑在 Rust 侧作为 `trylock_owner_impl` 私有函数实现，不作为独立导出符号，而是内联在 `__pthread_mutex_trylock` 调用路径中。

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
impl PthreadMutex {
    pub(crate) fn try_lock(&self) -> Result<(), core::ffi::c_int> {
        // 安全包装：非阻塞尝试加锁
        // 返回 Ok(()) 表示成功，Err(EBUSY) 表示已被锁定
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  atomic_cas (内部模块)               // 依赖: 原子比较交换，NORMAL 快速路径
  current_thread (内部模块)           // 依赖: 获取当前线程控制块和 tid
  linux_futex (内部模块)              // 依赖: PI futex 操作（FUTEX_UNLOCK_PI）
  linux_set_robust_list (内部模块)    // 依赖: 惰性注册 robust list
  core::sync::atomic::Ordering        // 依赖: 原子操作内存排序

[GUARANTEE]
Exported Interface:
  extern "C" fn __pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // musl 内部实现符号，rusl 必须导出
  extern "C" fn pthread_mutex_trylock(m: *mut pthread_mutex_t) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 pthread_mutex_trylock 符号
Internal Interface:
  impl PthreadMutex::try_lock(&self) -> Result<(), core::ffi::c_int>;
                                 // 安全包装，供 crate 内部使用
