# mtx_timedlock — Rust 接口归约

## 原始 C 接口
```c
int mtx_timedlock(mtx_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.4.4)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn mtx_timedlock(
    m: *mut mtx_t,
    ts: *const timespec,
) -> core::ffi::c_int;
```

---

## 意图
锁定互斥锁 `m`，若锁被占用则阻塞直至成功或超时。是 `__pthread_mutex_timedlock` 的包装器，将 POSIX 返回值映射为 C11 语义。当 `ts == null` 时表示无限期等待（musl 扩展），此扩展被 `mtx_lock` 利用以避免代码重复。

## 前置条件
- `m` 为非空指针（`!m.is_null()`），指向通过 `mtx_init` 初始化的互斥锁对象
- 互斥锁未被销毁
- 对于普通互斥锁：调用线程当前未持有该锁
- `ts` 指定了绝对时间点的超时（基于 `CLOCK_REALTIME`），或 `null` 表示无限期等待

## 后置条件
- Case 1 锁成功获取：返回 `thrd_success` (0)
- Case 2 超时到期且未获取锁：返回 `thrd_timedout` (4)，锁未被获取
- Case 3 其他错误（如 `EINVAL` 无效参数、`EAGAIN` 递归锁超过最大重入数等）：返回 `thrd_error` (2)

## 不变量
- 成功返回时调用线程独占互斥锁，重入计数按互斥锁类型正确更新

## 算法
```rust
extern "C" fn mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> c_int {
    let ret = unsafe {
        __pthread_mutex_timedlock(m as *mut pthread_mutex_t, ts)
    };
    match ret {
        0 => thrd_success,
        ETIMEDOUT => thrd_timedout,
        _ => thrd_error,
    }
}
```

内部 `__pthread_mutex_timedlock` 基于原子操作和 futex（`__timedwait`）实现，负责处理普通/递归/检错/健壮各种互斥锁类型。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 互斥锁定时加锁
pub(crate) unsafe fn __pthread_mutex_timedlock(
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_mutex_timedlock(m: *mut pthread_mutex_t, ts: *const timespec) -> c_int
                                      // 依赖1: musl 内部互斥锁定时加锁实现
Predefined Macros/Types:
  mtx_t (= pthread_mutex_t, repr(C))  // C11 互斥锁类型
  struct timespec                      // POSIX 时间结构体
  thrd_success (0) / thrd_timedout (4) / thrd_error (2)
  ETIMEDOUT                            // 超时 errno

[GUARANTEE]
Exported Interface:
  extern "C" fn mtx_timedlock(m: *mut mtx_t, ts: *const timespec) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 mtx_timedlock 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_mutex_timedlock(
      m: *mut pthread_mutex_t, ts: *const timespec
  ) -> c_int;
                                        // 内部互斥锁定时加锁，模块间共享
