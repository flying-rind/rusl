# tcgetpgrp — Rust 接口归约

## 原始 C 接口
```c
pid_t tcgetpgrp(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn tcgetpgrp(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图

获取与文件描述符 `fd` 关联的终端的前台进程组 ID。通过 `TIOCGPGRP` ioctl 控制命令实现。

## 前置条件

- `fd`: 有效的已打开文件描述符，必须关联到一个会话的控制终端

## 后置条件

- **Case 1 成功**
  - 返回与 `fd` 关联的终端的前台进程组 ID（`pid_t` 类型，非负整数）

- **Case 2 `fd` 不关联终端或 ioctl 失败**
  - 返回 -1
  - `errno` 由 `ioctl` 设置（如 `EBADF`、`ENOTTY`）

## 不变量

无。本函数不持有任何内部状态。

## 算法

原 C 实现通过 `ioctl(fd, TIOCGPGRP, &pgrp)` 获取前台进程组。Rust 中：

```rust
extern "C" fn tcgetpgrp(fd: core::ffi::c_int) -> core::ffi::c_int {
    let mut pgrp: core::ffi::c_int = 0;
    let ret = unsafe { sys_ioctl(fd, TIOCGPGRP, &mut pgrp) };
    if ret < 0 {
        -1
    } else {
        pgrp
    }
}
```

即：
1. 声明 `pgrp` 作为 ioctl 的输出缓冲区
2. 调用 `ioctl(fd, TIOCGPGRP, &pgrp)` — `TIOCGPGRP` 将终端的前台进程组 ID 写入 `pgrp`
3. 若 `ioctl` 返回负值（出错），函数返回 -1
4. 否则返回 `pgrp` 的值

---

## Rust 安全包装（模块内部）

```rust
/// 安全包装，返回 Result<i32, Errno>
pub(crate) fn get_foreground_process_group(fd: RawFd) -> Result<i32, Errno> {
    let ret = unsafe { tcgetpgrp(fd) };
    if ret >= 0 {
        Ok(ret)
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
  TIOCGPGRP — 获取前台进程组的 ioctl 请求码 (0x540F)，定义在 <sys/ioctl.h>
  pid_t — 进程 ID 类型
Predefined Macros/Crates:
  syscall 模块 (rusl 内部 syscall 封装)

[GUARANTEE]
Exported Interface:
  extern "C" fn tcgetpgrp(fd: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 tcgetpgrp 符号
Internal Interface:
  pub(crate) fn get_foreground_process_group(fd: RawFd) -> Result<i32, Errno>;
                                 // 安全 Rust 包装
