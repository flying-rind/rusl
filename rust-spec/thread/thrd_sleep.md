# thrd_sleep — Rust 接口归约

## 原始 C 接口
```c
int thrd_sleep(const struct timespec *req, struct timespec *rem);
```

[Visibility]: User — 通过 `<threads.h>` 对外导出 (ISO C11 7.26.5.6)

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn thrd_sleep(
    req: *const timespec,
    rem: *mut timespec,
) -> core::ffi::c_int;
```

---

## 意图
使调用线程阻塞至少 `req` 指定的时间。若阻塞被信号中断，剩余时间写入 `rem`。使用 `CLOCK_REALTIME` 时钟，即使系统实时时钟被调整，睡眠时长不受影响（通过 `TIMER_ABSTIME` 标志为 0 即相对时间实现）。

## 前置条件
- `req` 为非空指针（`!req.is_null()`），指定睡眠时长
- `req->tv_sec >= 0`，`req->tv_nsec` 在 `[0, 999999999]` 范围内
- `rem` 可为 `null`（不关心中断后剩余时间）或指向有效 `timespec`

## 后置条件
- Case 1 完整睡眠完成：调用线程已阻塞至少 `req` 时长，返回 0
- Case 2 被信号中断（`EINTR`）：睡眠提前结束。若 `rem` 非空，`*rem` 包含剩余未睡眠的时间。返回 -1
- Case 3 其他错误（参数无效等）：返回 -2

## 不变量
- 返回值 0 / -1 / -2 遵循 C11 标准（注意 *非* thrd_success/thrd_error 枚举值）
- 睡眠时长以 `CLOCK_REALTIME` 为基准

## 算法
```rust
// musl 内部 clock_nanosleep 返回负 errno 值
extern "C" fn thrd_sleep(req: *const timespec, rem: *mut timespec) -> c_int {
    unsafe {
        // __clock_nanosleep 内部返回负 errno 值（musl 约定）
        let ret = -__clock_nanosleep(CLOCK_REALTIME, 0, req, rem);
        // switch 映射返回值
        match ret {
            0 => 0,        // 完整睡眠
            -EINTR => -1,  // 信号中断
            _ => -2,       // 其他错误
        }
    }
}
```

注意：`thrd_sleep` 的返回值为特殊的整数值 0/-1/-2，**不是** C11 thrd_* 枚举值。

---

## Rust 内部辅助接口（模块私有）

```rust
// musl 内部时钟睡眠实现，返回负 errno 值
pub(crate) unsafe fn __clock_nanosleep(
    clk: clockid_t,
    flags: c_int,
    req: *const timespec,
    rem: *mut timespec,
) -> c_int;
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __clock_nanosleep(clk: clockid_t, flags: c_int, req: *const timespec, rem: *mut timespec) -> c_int
                                      // 依赖1: musl 内部时钟睡眠实现（返回负 errno）
Predefined Macros/Types:
  struct timespec { tv_sec: time_t, tv_nsec: c_long }  // POSIX 时间结构体 (repr(C))
  CLOCK_REALTIME (= 0)                 // 实时时钟 ID
  EINTR                                // 信号中断 errno

[GUARANTEE]
Exported Interface:
  extern "C" fn thrd_sleep(req: *const timespec, rem: *mut timespec) -> core::ffi::c_int;
                                        // 本模块保证对外提供与 C ABI 兼容的 thrd_sleep 符号
Internal Interface:
  pub(crate) unsafe fn __clock_nanosleep(
      clk: clockid_t, flags: c_int, req: *const timespec, rem: *mut timespec
  ) -> c_int;
                                        // 内部时钟睡眠，模块间共享
