# _exit — Rust 接口归约

## 原始 C 接口
```c
_Noreturn void _exit(int status);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn _exit(status: core::ffi::c_int) -> !;
```

---

## 意图
立即终止调用进程，不执行任何清理操作：
- 不调用 `atexit()` 或 `on_exit()` 注册的函数
- 不刷新 stdio 缓冲区
- 不删除临时文件
- 关闭所有打开的文件描述符（由内核完成）
- 子进程被 init（PID 1）收养

Rust 侧通过 `extern "C"` 导出符号，内部直接委托给 `_Exit` 标准函数或直接调用 `SYS_exit_group` 系统调用。

## 前置条件
- `status`: 进程退出状态码。仅低 8 位对父进程可见（通过 `wait()` 系列函数）。`status & 0xFF` 为有效退出状态

## 后置条件
- 调用进程终止（函数不返回，Rust 中返回类型为 `!`）
- `status & 0xFF` 成为进程的退出状态
- 为子进程生成 `SIGCHLD` 信号（若父进程未设置 `SA_NOCLDWAIT`）
- 若父进程调用 `wait()`，将收到此退出状态

## 不变量
无。本函数不返回，不持有任何内部状态。

## 算法
原 C 实现直接委托 `_Exit(status)`，后者最终调用 `SYS_exit_group` 系统调用。Rust 中：

```
_exit(status):
  _Exit(status)              // 委托给 ISO C _Exit，它调用 SYS_exit_group 系统调用
```

---

## Rust 安全包装（模块内部）

```rust
// 安全包装：标记为发散函数，调用者知道它不返回
pub(crate) fn exit_process(status: i32) -> ! {
    unsafe { sys_exit_group(status) }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_exit_group (syscall number)      // 依赖1: 底层 exit_group 系统调用
Predefined Macros/Crates:
  syscall! 宏或 libc::syscall          // 依赖2: 系统调用封装

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn _exit(status: core::ffi::c_int) -> !;
                                       // 本模块保证对外提供与 C ABI 兼容的 _exit 符号
Internal Interface:
  pub(crate) fn exit_process(status: i32) -> !;
                                       // 安全 Rust 包装，标记为发散函数
