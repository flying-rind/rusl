# __timedwait — Rust 接口归约

> rusl 内部带超时的 futex 等待函数。提供基于 futex 的带超时线程等待原语，支持取消点 (cancellation point) 检查。包括对 32 位/64 位时间系统调用的兼容处理。Rust 实现中使用 `Duration` 风格的时间抽象和 futex 系统调用。

## 原始 C 接口

```c
int __timedwait(volatile int *addr, int val, clockid_t clk, const struct timespec *at, int priv);
int __timedwait_cp(volatile int *addr, int val, clockid_t clk, const struct timespec *at, int priv);
```

[Visibility]: Internal — 被 `pthread_impl.h` 声明为 hidden，仅 musl/rusl 内部使用

---

## Rust 外部 ABI 接口

```rust
pub extern "C" fn __timedwait(
    addr: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    clk: clockid_t,
    at: *const libc_compat::timespec,
    priv: core::ffi::c_int,
) -> core::ffi::c_int;

pub extern "C" fn __timedwait_cp(
    addr: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    clk: clockid_t,
    at: *const libc_compat::timespec,
    priv: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 依赖图

```
__timedwait
  ├─> __pthread_setcancelstate()         (外部 — 取消状态管理)
  └─> __timedwait_cp()                   (本模块)

__timedwait_cp
  ├─> __clock_gettime()                  (外部 — POSIX 时钟)
  ├─> futex4_cp()                        (内部 static 函数)
  │     ├─> __syscall_cp(SYS_futex_time64) (外部 — 取消点系统调用)
  │     └─> __syscall_cp(SYS_futex)        (外部 — 取消点系统调用)
  └─> __eintr_valid_flag                 (弱别名全局变量)
```

---

## 内部辅助函数（不对外导出）

### futex4_cp

```rust
// crate 内部可见，不对外导出
pub(crate) fn futex4_cp(
    addr: *mut core::ffi::c_int,
    op: core::ffi::c_int,
    val: core::ffi::c_int,
    to: Option<&libc_compat::timespec>,
) -> core::ffi::c_int;
```

#### Intent

带取消点的 4 参数 futex 系统调用包装器。处理 64 位/32 位时间戳兼容性：优先尝试 `SYS_futex_time64`，若不支持则回退到 `SYS_futex` 并将时间戳钳制为 32 位。

Rust 实现中，使用 `Option<&timespec>` 替代 C 中的 NULL 指针判断，通过 `cfg(target_pointer_width)` 条件编译处理不同位宽的时间戳。

---

## 函数规约

### 1. __timedwait_cp

```rust
pub extern "C" fn __timedwait_cp(
    addr: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    clk: clockid_t,
    at: *const libc_compat::timespec,
    priv: core::ffi::c_int,
) -> core::ffi::c_int;
```

#### Intent

带取消点检查的、支持相对/绝对超时的 futex 等待函数。计算从当前时间到目标时间 `at` 的时间差，若已超时则立即返回 `ETIMEDOUT`。包含旧内核 `EINTR` 错误修正逻辑。

#### 前置条件

- `addr` 非空，指向有效的 futex 字
- `clk` 为有效的时钟 ID (`CLOCK_REALTIME` 或 `CLOCK_MONOTONIC`)
- `at` 可为 NULL（表示无限等待）
- 若 `at` 非空：`at->tv_nsec < 1000000000UL`
- `priv` 为 0 或非 0（非 0 表示私有 futex）

#### 后置条件

- Case `at == NULL`：无限等待，直到被唤醒或取消
- Case `at` 已过期：返回 `ETIMEDOUT`
- Case `at->tv_nsec >= 1000000000UL`：返回 `EINVAL`
- Case `__clock_gettime` 失败：返回 `EINVAL`
- Case futex 返回 `EINTR` 且 `__eintr_valid_flag` 为假：修正为返回 0
- 返回 0 表示被正常唤醒，非零值表示错误/超时

#### 系统算法

```
__timedwait_cp(addr, val, clk, at, priv):
  1. if priv != 0: priv = FUTEX_PRIVATE
  2. if at.is_some():
  3.   if at.tv_nsec >= 1_000_000_000: return EINVAL
  4.   if __clock_gettime(clk, &to) != 0: return EINVAL
  5.   to.tv_sec = at.tv_sec - to.tv_sec
  6.   to.tv_nsec = at.tv_nsec - to.tv_nsec
  7.   if to.tv_nsec < 0: to.tv_sec -= 1; to.tv_nsec += 1_000_000_000
  8.   if to.tv_sec < 0: return ETIMEDOUT
  9. r = -futex4_cp(addr, FUTEX_WAIT | priv, val, Some(&to))
 10. if r != EINTR && r != ETIMEDOUT && r != ECANCELED: r = 0
 11. if r == EINTR && !__eintr_valid_flag: r = 0
 12. return r
```

---

### 2. __timedwait

```rust
pub extern "C" fn __timedwait(
    addr: *mut core::ffi::c_int,
    val: core::ffi::c_int,
    clk: clockid_t,
    at: *const libc_compat::timespec,
    priv: core::ffi::c_int,
) -> core::ffi::c_int;
```

#### Intent

`__timedwait_cp` 的无取消点包装器。在调用 `__timedwait_cp` 期间暂时禁用线程取消，确保超时等待过程不会被取消中断。

#### 前置条件

- 与 `__timedwait_cp` 相同
- 调用者处于有效环境中（可调用 `__pthread_setcancelstate`）

#### 后置条件

- 调用期间取消被禁用，返回前恢复原始取消状态
- 返回值与 `__timedwait_cp` 相同
- 即使等待期间有取消请求挂起，也会完成等待并返回结果

#### 系统算法

```
__timedwait(addr, val, clk, at, priv):
  1. cs = __pthread_setcancelstate(PTHREAD_CANCEL_DISABLE)
  2. r = __timedwait_cp(addr, val, clk, at, priv)
  3. __pthread_setcancelstate(cs)
  4. return r
```

---

## Rust 安全包装（模块内部，不对外暴露）

```rust
use core::time::Duration;

// 安全的超时等待抽象
pub(crate) fn timed_wait(
    addr: &core::sync::atomic::AtomicI32,
    val: i32,
    clk: ClockId,
    timeout: Option<Duration>,
    private: bool,
) -> Result<(), WaitError> {
    // 内部通过 futex 系统调用实现
    // ...
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  __clock_gettime()                       // 依赖1: POSIX 获取当前时间
  __syscall_cp()                          // 依赖2: 带取消点的系统调用
  __pthread_setcancelstate()             // 依赖3: 取消状态管理
  __eintr_valid_flag                      // 依赖4: EINTR 有效性标志
  SYS_futex / SYS_futex_time64           // 依赖5: futex 系统调用号

[GUARANTEE]
Exported Interface:
  pub extern "C" fn __timedwait(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;
  pub extern "C" fn __timedwait_cp(addr: *mut c_int, val: c_int, clk: clockid_t, at: *const timespec, priv: c_int) -> c_int;
                                         // 本模块保证对外提供与 C ABI 兼容的符号
Internal Interface:
  pub(crate) fn futex4_cp(addr: *mut c_int, op: c_int, val: c_int, to: Option<&timespec>) -> c_int;
  pub(crate) fn timed_wait(addr: &AtomicI32, val: i32, clk: ClockId, timeout: Option<Duration>, private: bool) -> Result<(), WaitError>;
                                         // 安全包装，供 crate 内部使用
