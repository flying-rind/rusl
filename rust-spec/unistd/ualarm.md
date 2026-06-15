# ualarm — Rust 接口归约

## 原始 C 接口
```c
unsigned ualarm(unsigned value, unsigned interval);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn ualarm(value: core::ffi::c_uint, interval: core::ffi::c_uint) -> core::ffi::c_uint;
```

---

## 意图
设置一个真实时间闹钟，在 `value` 微秒后第一次触发 `SIGALRM`，之后每隔 `interval` 微秒重复触发。如果之前已经设置过定时器，则取消之前的定时器。调用 `ualarm(0, 0)` 取消任何待处理的定时器。

与 `alarm()`（秒级分辨率，无重复间隔）相比，`ualarm` 提供了微秒级精度和自动重复能力。该实现基于 `setitimer(ITIMER_REAL, ...)` 构建。

## 前置条件
- `value`: 第一次触发前的等待时间（微秒）。0 表示不设置首次触发
- `interval`: 首次触发后每次重复触发的间隔（微秒）。0 表示只触发一次

## 后置条件
- **Case 1 之前没有定时器**
  - 设置新的间隔定时器
  - 返回 0

- **Case 2 之前有定时器（被覆盖）**
  - 之前的定时器被取消
  - 设置新的间隔定时器
  - 返回之前定时器距离下次触发所剩余的微秒数

- **Case 3 `value == 0 && interval == 0`（取消定时器）**
  - 取消任何待处理的间隔定时器
  - 返回之前定时器剩余微秒数

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现基于 `setitimer(ITIMER_REAL, ...)` 构建。Rust 中：

```
ualarm(value, interval):
  it.it_interval.tv_usec = interval                     // 1. 设置重复间隔（微秒）
  it.it_value.tv_usec    = value                        // 2. 设置首次触发延迟（微秒）
  setitimer(ITIMER_REAL, &it, &it_old)                  // 3. 设置真实时间间隔定时器
  return it_old.it_value.tv_sec * 1000000               // 4. 返回旧定时器剩余微秒数
       + it_old.it_value.tv_usec                        //    = tv_sec * 10^6 + tv_usec
```

---

## Rust 安全包装（模块内部）

```rust
/// 设置微秒级间隔闹钟。
/// 返回之前定时器剩余微秒数，0 表示之前无定时器。
pub(crate) fn set_ualarm(value: u32, interval: u32) -> u32 {
    // 内部通过 setitimer(ITIMER_REAL, ...) 实现
    // ...
}
```

---

## 注意事项
- `ualarm` 具有可移植性问题：它是 BSD/SVID 扩展，在 POSIX.1-2001 中标记为已废弃，在 POSIX.1-2008 中被移除。推荐使用 `setitimer` 或 POSIX 定时器代替
- 与 `alarm()` 共享同一个 `ITIMER_REAL` 定时器，因此调用 `ualarm` 会影响调用 `alarm` 设置的状态，反之亦然

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  setitimer(int, const struct itimerval *, struct itimerval *)  // 依赖1: 设置间隔定时器
  ITIMER_REAL                         // 依赖2: 真实时间定时器类型
  struct itimerval                    // 依赖3: 间隔定时器值结构
Predefined Macros/Crates:
  libc crate（或等效 no_std crate）   // 依赖4: 提供 setitimer 和 ITIMER_REAL

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn ualarm(value: core::ffi::c_uint, interval: core::ffi::c_uint) -> core::ffi::c_uint;
                                       // 本模块保证对外提供与 C ABI 兼容的 ualarm 符号
Internal Interface:
  pub(crate) fn set_ualarm(value: u32, interval: u32) -> u32;
                                       // 安全 Rust 包装
