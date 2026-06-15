# alarm — Rust 接口归约

## 原始 C 接口
```c
unsigned alarm(unsigned seconds);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn alarm(seconds: core::ffi::c_uint) -> core::ffi::c_uint;
```

---

## 意图
设置一个真实时间闹钟，在 `seconds` 秒后向调用进程发送 `SIGALRM` 信号。如果之前已经设置过闹钟且尚未触发，则取消之前的闹钟并返回剩余秒数。调用 `alarm(0)` 取消任何待处理的闹钟，且不发送 `SIGALRM`。

该实现基于 `setitimer(ITIMER_REAL, ...)` 构建，而非直接使用 `SYS_alarm` 系统调用。这样可以获得更高的精度（微秒级）以及更一致的接口语义。

## 前置条件
- `seconds`: 触发闹钟前需要等待的秒数。0 表示取消之前的闹钟

## 后置条件
- **Case 1 之前没有待处理的闹钟**
  - 设置新的闹钟，在 `seconds` 秒后触发 `SIGALRM`
  - 返回 0

- **Case 2 之前有待处理的闹钟（被覆盖）**
  - 之前的闹钟被取消
  - 设置新的闹钟，在 `seconds` 秒后触发 `SIGALRM`
  - 返回之前闹钟剩余的秒数（向上取整到最近的秒数）

- **Case 3 `seconds == 0`（取消闹钟）**
  - 取消任何待处理的闹钟
  - 不发送 `SIGALRM`
  - 返回之前闹钟剩余的秒数（向上取整）

## 算法
原 C 实现基于 `setitimer(ITIMER_REAL, ...)` 构建。Rust 中：

```
alarm(seconds):
  it.it_value.tv_sec = seconds      // 1. 设置定时器初值 (秒)
  it.it_value.tv_usec = 0           // 2. 微秒部分为0
  setitimer(ITIMER_REAL, &it, &old) // 3. 设置真实时间定时器
  return old.it_value.tv_sec        // 4. 返回旧定时器剩余秒数
       + (old.it_value.tv_usec != 0) as u32  // 若微秒部分非0则+1（向上取整）
```

---

## Rust 安全包装（模块内部）

```rust
/// 设置 SIGALRM 闹钟。
/// 返回之前闹钟的剩余秒数，0 表示之前无闹钟。
pub(crate) fn set_alarm(seconds: u32) -> u32 {
    // 内部通过 setitimer(ITIMER_REAL, ...) 实现
    // ...
}
```

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
  unsafe extern "C" fn alarm(seconds: core::ffi::c_uint) -> core::ffi::c_uint;
                                       // 本模块保证对外提供与 C ABI 兼容的 alarm 符号
Internal Interface:
  pub(crate) fn set_alarm(seconds: u32) -> u32;
                                       // 安全 Rust 包装
