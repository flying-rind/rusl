# pause — Rust 接口归约

## 原始 C 接口
```c
int pause(void);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn pause() -> core::ffi::c_int;
```

---

## 意图
阻塞调用进程，直到收到一个信号并被信号处理器捕获，或者进程被该信号终止。`pause` 仅在信号处理器返回后才返回，此时返回值为 -1 且 `errno` 被设置为 `EINTR`。

该实现使用可被信号取消的内部封装，使 `pause` 成为一个取消点（cancellation point），允许线程在被 `pthread_cancel` 取消时安全退出。

## 前置条件
- 无

## 后置条件
- **Case 1 被信号处理器中断（正常情况）**
  - 信号处理器已被调用并返回
  - 返回 -1
  - `errno` 设置为 `EINTR`

- **Case 2 被信号终止**
  - 进程终止（函数不返回）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现直接调用 `SYS_pause` 系统调用。Rust 中：

```
pause():
  return syscall(SYS_pause)        // 调用 pause 系统调用
```

在 Linux 上，`SYS_pause` 的行为是：内核挂起调用进程，直到收到一个信号并被信号处理器捕获，然后返回 `-EINTR`。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_pause (syscall number)          // 依赖1: 底层 pause 系统调用
Predefined Macros/Crates:
  syscall! 宏或等效 no_std 封装      // 依赖2: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn pause() -> core::ffi::c_int;
                                       // 本模块保证对外提供与 C ABI 兼容的 pause 符号
