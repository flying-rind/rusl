# sleep — Rust 接口归约

## 原始 C 接口
```c
unsigned sleep(unsigned seconds);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn sleep(seconds: core::ffi::c_uint) -> core::ffi::c_uint;
```

---

## 意图
使调用进程暂停执行 `seconds` 秒。如果睡眠被信号处理器中断，则返回剩余的未睡眠秒数。

该实现基于 `nanosleep()` 构建（而非 `SYS_alarm` + `SYS_pause` 或 `SYS_nanosleep` 系统调用），使用 `struct timespec` 实现精确的秒级睡眠。若 `nanosleep` 被信号中断，`tv` 参数（传入时同时作为输出）将被内核更新为剩余时间，`sleep` 直接返回剩余秒数。

## 前置条件
- `seconds`: 要睡眠的秒数。0 表示不睡眠（但可能让出 CPU）

## 后置条件
- **Case 1 完整睡眠**
  - 调用进程在至少 `seconds` 秒内未执行
  - 实际睡眠时间可能因系统负载和时钟粒度而略长
  - 返回 0

- **Case 2 被信号处理器中断**
  - 返回剩余未睡眠的秒数
  - 调用者可选择再次调用 `sleep(result)` 以完成剩余睡眠

- **Case 3 `seconds == 0`**
  - 立即返回 0（不睡眠，但可能触发上下文切换）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现基于 `nanosleep()` 构建，使用同一 `tv` 变量同时作为请求和剩余时间参数。Rust 中：

```
sleep(seconds):
  tv.tv_sec  = seconds            // 1. 设置秒级睡眠时长
  tv.tv_nsec = 0                  // 2. 纳秒部分为0
  if nanosleep(&tv, &tv) < 0:     // 3. 调用 nanosleep 进行高精度睡眠
    return tv.tv_sec              // 4. 被信号中断：返回剩余秒数
  return 0                        // 5. 完整睡眠：返回0
```

注意：`nanosleep` 被信号中断时，内核将剩余时间写回 `tv`，然后 `sleep` 直接读取 `tv.tv_sec` 返回。

---

## Rust 安全包装（模块内部）

```rust
/// 睡眠指定秒数，返回剩余未睡眠的秒数（0 表示完整睡眠）。
pub(crate) fn sleep_seconds(seconds: u32) -> u32 {
    // 内部通过 nanosleep 实现
    // ...
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  nanosleep(const struct timespec *, struct timespec *)  // 依赖1: 高精度睡眠
  struct timespec                     // 依赖2: POSIX 时间规范结构
Predefined Macros/Crates:
  libc crate（或等效 no_std crate）   // 依赖3: 提供 nanosleep

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn sleep(seconds: core::ffi::c_uint) -> core::ffi::c_uint;
                                       // 本模块保证对外提供与 C ABI 兼容的 sleep 符号
Internal Interface:
  pub(crate) fn sleep_seconds(seconds: u32) -> u32;
                                       // 安全 Rust 包装
