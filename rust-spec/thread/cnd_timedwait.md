# cnd_timedwait — Rust 接口归约

## 原始 C 接口
```c
int cnd_timedwait(cnd_t *restrict c, mtx_t *restrict m, const struct timespec *restrict ts);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.3.6)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
// Rust 没有 restrict 关键字，通过文档约定指针无别名
extern "C" fn cnd_timedwait(
    c: *mut cnd_t,
    m: *mut mtx_t,
    ts: *const timespec,
) -> core::ffi::c_int;
```

---

## 意图
原子地释放互斥锁 `m` 并在条件变量 `c` 上阻塞调用线程，直到被 `cnd_signal` / `cnd_broadcast` 唤醒或超时到期。被唤醒后重新获取 `m`。是 `__pthread_cond_timedwait` 的包装器，负责将 POSIX 返回值映射为 C11 返回值。

## 前置条件
- `c` 为非空指针，指向通过 `cnd_init` 初始化的条件变量
- `m` 为非空指针，指向通过 `mtx_init` 初始化的互斥锁
- 调用线程必须已锁定 `m`
- `ts` 指定了绝对时间点的超时（基于 `CLOCK_REALTIME`），或为 `null` 表示无限期等待（musl 扩展）

## 后置条件
- 返回前互斥锁 `m` 已被调用线程重新锁定
- Case 1 被 `cnd_signal` / `cnd_broadcast` 唤醒：返回 `thrd_success` (0)
- Case 2 超时到期且未被唤醒：返回 `thrd_timedout` (4)
- Case 3 其他错误（如 `EINVAL` 无效参数、`EPERM` 互斥锁不归当前线程所有）：返回 `thrd_error` (2)

## 不变量
- 调用线程在阻塞期间不持有互斥锁 `m`，返回时重新持有

## 算法
```rust
extern "C" fn cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> c_int {
    let ret = unsafe {
        // 将 cnd_t* 和 mtx_t* 转换为底层 pthread 类型
        __pthread_cond_timedwait(c as *mut pthread_cond_t, m as *mut pthread_mutex_t, ts)
    };
    match ret {
        0 => thrd_success,
        ETIMEDOUT => thrd_timedout,
        _ => thrd_error,
    }
}
```

内部 `__pthread_cond_timedwait` 基于 futex 实现带超时的条件变量等待，负责原子性释放锁 + 阻塞 + 重新获取锁的全部逻辑。

---

## Rust 内部辅助接口（模块私有）

```rust
// 内部 POSIX 条件变量定时等待
pub(crate) unsafe fn __pthread_cond_timedwait(
    c: *mut pthread_cond_t,
    m: *mut pthread_mutex_t,
    ts: *const timespec,
) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int
                                      // 依赖1: musl 内部条件变量定时等待实现
Predefined Macros/Types:
  cnd_t (= pthread_cond_t)             // C11 条件变量类型
  mtx_t (= pthread_mutex_t)            // C11 互斥锁类型
  struct timespec                      // POSIX 时间结构体
  thrd_success (0) / thrd_timedout (4) / thrd_error (2)
  ETIMEDOUT                            // 超时 errno

[GUARANTEE]
Exported Interface:
  extern "C" fn cnd_timedwait(c: *mut cnd_t, m: *mut mtx_t, ts: *const timespec) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 cnd_timedwait 符号
Internal Interface:
  pub(crate) unsafe fn __pthread_cond_timedwait(
      c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec
  ) -> c_int;
                                        // 内部条件变量定时等待，模块间共享
