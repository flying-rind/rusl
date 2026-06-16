# pthread_cond_broadcast -- Rust 接口归约

## 原始 C 接口
```c
int pthread_cond_broadcast(pthread_cond_t *c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
唤醒所有当前阻塞在条件变量 `c` 上的线程。对进程内条件变量使用内部链表机制；对进程共享条件变量使用 `_c_seq` 计数器 + futex 唤醒。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向有效的 `pthread_cond_t` 对象
- 条件变量已通过 `pthread_cond_init` 初始化

## 后置条件
- **进程内（非共享）条件变量**：委托 `__private_cond_signal(c, -1)` 处理，唤醒等待链表上所有 waiter
- **进程共享条件变量**：
  - Case 1 有等待者（`_c_waiters > 0`）：递增 `_c_seq` 并调用 `wake(&_c_seq, -1, 0)` 唤醒所有等待者，返回 `0`
  - Case 2 无等待者（`_c_waiters == 0`）：直接返回 `0`
- 始终返回 `0`

## 不变量
- 进程共享模式下，`_c_seq` 递增是广播的信号，保证所有等待者被唤醒
- `__private_cond_signal` 的 `n = -1` 语义为"唤醒全部"

## 算法
```rust
pub extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int {
    unsafe {
        let cond = &*c;
        if cond.c_shared().is_null() {
            // 进程内条件变量：委托内部链表广播
            return __private_cond_signal(c, -1);
        }
        if cond.c_waiters().load(Ordering::Relaxed) == 0 {
            return 0;  // 无等待者
        }
        cond.c_seq().fetch_add(1, Ordering::Release);
        wake(cond.c_seq().as_ptr(), -1, 0);  // cnt=-1 唤醒全部等待者
    }
    0
}
```

## Rust 内部设计要点
- `PthreadCond` 提供 `c_seq()`, `c_waiters()`, `c_shared()` 字段访问
- `__private_cond_signal` 为内部函数，`n = -1` 表示广播全部
- `wake()` 为 futex 唤醒函数，`cnt = -1` 表示唤醒所有等待者，定义于 `pthread_impl` 模块
- 与 `pthread_cond_signal` 的区别仅为：`n` 参数值（1 vs -1）和 `wake` 的 `cnt` 参数值

---

/* Rely */
[RELY]
Predefined Types:
  pthread_cond_t                   // #[repr(C)] 条件变量类型
  core::ffi::c_int                // Rust 核心库 C FFI 类型

Internal Module:
  pthread_impl::PthreadCond        // 字段访问（c_seq, c_waiters, c_shared）
  pthread_impl::wake               // futex 唤醒

Internal Functions (同一 crate):
  __private_cond_signal            // 进程内条件变量链表信号处理，n=-1 表示全部

Rust Core:
  core::sync::atomic::Ordering     // 原子操作内存序

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
