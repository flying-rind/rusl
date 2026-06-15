# tcsetpgrp — Rust 接口归约

## 原始 C 接口
```c
int tcsetpgrp(int fd, pid_t pgrp);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn tcsetpgrp(fd: core::ffi::c_int, pgrp: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

将文件描述符 `fd` 关联的终端的前台进程组设置为 `pgrp`。通过 `TIOCSPGRP` ioctl 控制命令实现。调用进程必须与终端属于同一会话。

## 前置条件

- `fd`: 有效的已打开文件描述符，必须关联到调用进程所在会话的控制终端
- `pgrp`: 目标前台进程组 ID，必须与调用进程属于同一会话
- 调用进程必须持有终端的控制权

## 后置条件

- **Case 1 成功**
  - 终端的前台进程组被设置为 `pgrp`
  - 返回 0

- **Case 2 失败**
  - 返回 -1
  - `errno` 由 `ioctl` 设置（如 `EBADF`、`ENOTTY`、`EPERM`、`EINVAL`）

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现通过 `ioctl(fd, TIOCSPGRP, &pgrp_int)` 设置前台进程组。Rust 中：

```rust
extern "C" fn tcsetpgrp(fd: core::ffi::c_int, pgrp: core::ffi::c_int) -> core::ffi::c_int {
    // pgrp 已经是 c_int 类型，直接传递
    unsafe { sys_ioctl(fd, TIOCSPGRP, &pgrp) }
}
```

即：
1. `pgrp` 参数已是 `c_int` 类型，直接作为 ioctl 参数
2. 调用 `ioctl(fd, TIOCSPGRP, &pgrp)` — `TIOCSPGRP` 将终端的前台进程组设置为 `pgrp` 的值
3. 直接返回 ioctl 的结果（成功 0，失败 -1）

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 Result<(), Errno>
pub(crate) fn set_foreground_process_group(fd: RawFd, pgrp: i32) -> Result<(), Errno> {
    let ret = unsafe { tcsetpgrp(fd, pgrp) };
    if ret == 0 {
        Ok(())
    } else {
        Err(Errno::from_last())
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  ioctl — 设备控制操作
  TIOCSPGRP — 设置前台进程组的 ioctl 请求码 (0x5410)，定义在 <sys/ioctl.h>
  pid_t — 进程 ID 类型
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn tcsetpgrp(fd: core::ffi::c_int, pgrp: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 tcsetpgrp 符号
Internal Interface:
  pub(crate) fn set_foreground_process_group(fd: RawFd, pgrp: i32) -> Result<(), Errno>;
                                 // 安全 Rust 包装
