# mtx_lock — Rust 接口归约

## 原始 C 接口
```c
int mtx_lock(mtx_t *m);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.3)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_lock(m: *mut mtx_t) -> core::ffi::c_int;
```

---

## 意图
锁定互斥锁 `m`。若互斥锁已被其他线程持有，则阻塞直到锁可用。采用两阶段策略：对普通互斥锁尝试原子快速路径，若失败则委托 `mtx_timedlock`（以 null 时间戳表示无限等待）。对递归互斥锁直接委托给 `mtx_timedlock` 处理重入逻辑。

## 前置条件
- `m` 为非空指针（`!m.is_null()`），指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁
- 对于普通互斥锁 (`mtx_plain`)：调用线程当前未持有该锁（普通互斥锁不可重入）
- 对于递归互斥锁 (`mtx_recursive`)：调用线程可重入（需配对 `mtx_unlock`）

## 后置条件
- Case 1 普通互斥锁未被占用（`m->_m_lock == 0`）：CAS 将 `_m_lock` 从 0 原子设为 `EBUSY`，返回 `thrd_success` (0)
- Case 2 普通互斥锁已被占用（`m->_m_lock != 0`）：CAS 失败，委托 `mtx_timedlock(m, null)` 阻塞等待锁释放
- Case 3 递归互斥锁 (`m->_m_type != PTHREAD_MUTEX_NORMAL`)：直接调用 `mtx_timedlock(m, null)` 处理
- 返回值为 `mtx_timedlock` 的返回值或 `thrd_success`

## 不变量
- 调用线程在返回时独占互斥锁（对于普通互斥锁）
- 对于递归互斥锁，重入计数在内部正确维护

## 算法
```rust
extern "C" fn mtx_lock(m: *mut mtx_t) -> c_int {
    unsafe {
        // 快速路径：普通互斥锁 + 尝试 CAS
        if (*m).m_type() == PTHREAD_MUTEX_NORMAL {
            // a_cas: 若 *p == 0 则 *p = EBUSY，返回旧值
            let old = a_cas(&raw mut (*m).m_lock() as *mut c_int, 0, EBUSY);
            if old == 0 {
                return thrd_success;
            }
        }
    }
    // 慢速路径：锁被占用或为非普通互斥锁，委托 mtx_timedlock
    // null 时间戳表示无限期等待（musl 扩展）
    mtx_timedlock(m, core::ptr::null())
}
```

注意：`a_cas(p, t, s)` 是 musl 内部原子 compare-and-swap 操作。在 Rust 中，可用 `core::sync::atomic::AtomicI32::compare_exchange` 替代，但需确保与 C ABI 的 `EBUSY` 值一致。

---

## Rust 内部辅助接口（模块私有）

```rust
// 原子 CAS 操作（内部，需与 C 的 a_cas 语义一致）
pub(crate) unsafe fn a_cas(p: *mut c_int, t: c_int, s: c_int) -> c_int;

// 等价于:
// use core::sync::atomic::{AtomicI32, Ordering};
// (*(p as *const AtomicI32)).compare_exchange(t, s, Ordering::Acquire, Ordering::Relaxed)
// 返回旧值（用 unwrap_or 获取）
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  a_cas(p: *mut c_int, t: c_int, s: c_int) -> c_int  // 依赖1: 原子 compare-and-swap
  mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int  // 依赖2: 带超时加锁
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t)            // C11 互斥锁类型
  PTHREAD_MUTEX_NORMAL (0)             // 普通互斥锁类型常量
  EBUSY                                // 互斥锁"已锁定"标记 (用作锁状态值)
  thrd_success (= 0)                   // C11 成功返回值
  _m_type / _m_lock                    // 互斥锁结构字段访问

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_lock(m: *mut mtx_t) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 mtx_lock 符号
Internal Interface:
  pub(crate) unsafe fn a_cas(p: *mut c_int, t: c_int, s: c_int) -> c_int;
                                        // 内部原子操作
