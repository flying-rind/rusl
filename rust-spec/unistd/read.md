# read — Rust 接口归约

## 原始 C 接口
```c
ssize_t read(int fd, void *buf, size_t count);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn read(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, count: usize) -> isize;
```

---

## 意图
从文件描述符 `fd` 中读取至多 `count` 字节到缓冲区 `buf` 中。该调用是 `SYS_read` 系统调用的薄封装，使用 `syscall_cp` 确保可以被信号安全取消。

## 前置条件
- `fd`: 已打开的、以读模式（或读/写模式）打开的有效文件描述符（`c_int`）
- `buf`: 指向至少 `count` 字节可用内存的非空指针（`*mut c_void`）
- `count`: `[0, SSIZE_MAX]` 范围内的字节数（`usize`）

## 后置条件
- **Case 1 成功读取**: 返回实际读取的字节数（`0..=count`），0 表示 EOF
- **Case 2 被信号中断且未读取任何数据**: 返回 `-1`，`errno` 设置为 `EINTR`
- **Case 3 其他错误**: 返回 `-1`，`errno` 设置为对应错误码（`EBADF`、`EFAULT`、`EIO` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现直接委托 `syscall_cp(SYS_read, fd, buf, count)`。Rust 中：

```rust
// 方案：内联 syscall（no_std 环境）
#[inline]
unsafe fn sys_read(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, count: usize) -> isize {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_read, fd, buf, count)
    // aarch64: svc #0 with x8 = SYS_read
}
```

由于 POSIX `read` 的取消点语义（`syscall_cp`），Rust 实现需确保系统调用可被信号安全取消。在 `no_std` 环境下，取消点语义为可选特性，不在最小实现中强制。

---

## Rust 安全包装（模块内部）

```rust
// 安全包装，接受 &mut [u8] 切片
pub(crate) fn read_bytes(fd: core::ffi::c_int, buf: &mut [u8]) -> Result<usize, Error> {
    let nread = unsafe { read_syscall(fd, buf.as_mut_ptr() as *mut core::ffi::c_void, buf.len()) };
    if nread < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(nread as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_read                                    // 依赖1: Linux 系统调用编号 (x86_64: 0, aarch64: 63)
Predefined Macros/Crates:
  syscall! 宏或等价的 syscall 封装            // 依赖2: 系统调用入口

[GUARANTEE]
Exported Interface:
  extern "C" fn read(fd: core::ffi::c_int, buf: *mut core::ffi::c_void, count: usize) -> isize;
                                   // 本模块保证对外提供与 C ABI 兼容的 read 符号
Internal Interface:
  pub(crate) fn read_bytes(fd: core::ffi::c_int, buf: &mut [u8]) -> Result<usize, Error>;
                                   // 安全 Rust 包装
