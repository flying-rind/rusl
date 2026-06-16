# pthread_cond_destroy -- Rust 接口归约

## 原始 C 接口
```c
int pthread_cond_destroy(pthread_cond_t *c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
销毁条件变量。对于进程共享的条件变量，如果有等待者正在等待，则广播唤醒所有等待者并阻塞直到它们全部退出等待状态，确保安全销毁。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向有效的 `pthread_cond_t` 对象
- 无任何线程正在调用 `pthread_cond_wait` 或 `pthread_cond_timedwait` 使用该条件变量（除了当前正在等待且即将被销毁过程唤醒的线程）

## 后置条件
- **非进程共享条件变量**（`_c_shared == null`）：立即返回 `0`（依赖调用者保证无等待者）
- **进程共享条件变量**（`_c_shared == (void*)-1`）：
  - Case 1 有等待者（`_c_waiters > 0`）：
    - 在 `_c_waiters` 上设置标志位 `0x8000_0000` 标记"销毁中"
    - 递增 `_c_seq` 并 futex 广播唤醒所有等待者
    - 忙等直到 `_c_waiters` 低 31 位降到 0（所有等待者已退出）
    - 返回 `0`
  - Case 2 无等待者：立即返回 `0`
- 始终返回 `0`

## 不变量
- 销毁过程必须等待所有正在等待的线程安全退出后才返回
- `_c_waiters` 的 `0x8000_0000` 位充当销毁信号，等待者线程在 `__pthread_cond_timedwait` 中通过 `a_fetch_add(&c->_c_waiters, -1) == -0x7fff_ffff` 检测到销毁并加速清理

## 算法
```rust
pub extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int {
    unsafe {
        let cond = &*c;
        if cond.c_shared().is_null() {
            return 0;  // 非进程共享，无需清理
        }
        let waiters = cond.c_waiters().load(Ordering::Relaxed);
        if waiters & 0x7FFF_FFFF == 0 {
            return 0;  // 无等待者
        }
        // 设置销毁标志
        cond.c_waiters().fetch_or(0x8000_0000, Ordering::Release);
        // 广播唤醒
        cond.c_seq().fetch_add(1, Ordering::Release);
        wake(cond.c_seq().as_ptr(), -1, 0);
        // 忙等所有等待者退出
        let mut v = cond.c_waiters().load(Ordering::Acquire);
        while (v & 0x7FFF_FFFF) != 0 {
            __wait(cond.c_waiters().as_ptr(), 0, v, 0);
            v = cond.c_waiters().load(Ordering::Acquire);
        }
    }
    0
}
```

## Rust 内部设计要点
- `PthreadCond` 提供 `c_seq()`, `c_waiters()`, `c_clock()`, `c_lock()`, `c_head()`, `c_tail()`, `c_shared()` 类型安全的字段访问方法
- 原子操作使用 `core::sync::atomic::Ordering` 替代 C 的隐式内存序
- `wake()` 和 `__wait()` 为 futex 操作函数，定义于 `pthread_impl` 模块
- `fetch_or`/`fetch_add` 替代 C 的 `a_or`/`a_inc` 原子宏

---

/* Rely */
[RELY]
Predefined Types:
  pthread_cond_t                   // #[repr(C)] 条件变量类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadCond        // 字段访问方法（c_seq, c_waiters, c_clock, c_lock, c_head, c_tail, c_shared）
  pthread_impl::wake               // futex 唤醒
  pthread_impl::__wait             // futex 等待

Rust Core:
  core::sync::atomic::{Ordering, AtomicI32}
                                   // 原子操作，替代 C 的 a_or / a_inc

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
