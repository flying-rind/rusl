# pthread_cond_signal -- Rust 接口归约

## 原始 C 接口
```c
int pthread_cond_signal(pthread_cond_t *c);
```

---

## Rust 外部 ABI 接口

```rust
extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> core::ffi::c_int;
```

[Visibility]: User — 通过 `<pthread.h>` 对外导出 (POSIX)

---

## 意图
唤醒至少一个当前阻塞在条件变量 `c` 上的线程。对进程内条件变量，通过链表机制精确唤醒一个 waiter；对进程共享条件变量，通过 `_c_seq` 计数器 + futex 唤醒一个等待者。

## 前置条件
- `c` 为非空指针（`!c.is_null()`），指向有效的 `pthread_cond_t` 对象
- 条件变量已通过 `pthread_cond_init` 初始化

## 后置条件
- **进程内（非共享）条件变量**：委托 `__private_cond_signal(c, 1)` 处理，从链表尾部取出一个 waiter 并唤醒
- **进程共享条件变量**：
  - Case 1 有等待者（`_c_waiters > 0`）：递增 `_c_seq` 并调用 `wake(&_c_seq, 1, 0)` 唤醒恰好一个等待者，返回 `0`
  - Case 2 无等待者（`_c_waiters == 0`）：直接返回 `0`
- 始终返回 `0`

## 不变量
- 每次 signal 至多唤醒一个等待者
- `_c_seq` 递增是唤醒信号，等待者通过比较 `_c_seq` 检测虚假唤醒

## 算法
```rust
pub extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int {
    unsafe {
        let cond = &*c;
        if cond.c_shared().is_null() {
            // 进程内条件变量：委托内部链表信号
            return __private_cond_signal(c, 1);
        }
        if cond.c_waiters().load(Ordering::Relaxed) == 0 {
            return 0;  // 无等待者
        }
        cond.c_seq().fetch_add(1, Ordering::Release);
        wake(cond.c_seq().as_ptr(), 1, 0);  // 唤醒恰好 1 个等待者
    }
    0
}
```

## Rust 内部设计要点
- `PthreadCond` 提供 `c_seq()`, `c_waiters()`, `c_shared()` 字段访问
- `__private_cond_signal` 为内部函数（定义于 `pthread_cond_timedwait` 对应模块），处理进程内链表信号
- `wake()` 为 futex 唤醒函数，定义于 `pthread_impl` 模块
- 原子递增使用 `fetch_add(1, Ordering::Release)` 确保释放语义

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
  __private_cond_signal            // 进程内条件变量链表信号处理，n=1 表示单个

Rust Core:
  core::sync::atomic::Ordering     // 原子操作内存序

[GUARANTEE]
Exported Interface:
  extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int;
                                   // 本模块保证对外提供与 C ABI 兼容的符号
