# write — Rust 接口归约

## 原始 C 接口
```c
ssize_t write(int fd, const void *buf, size_t count);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
extern "C" fn write(fd: core::ffi::c_int, buf: *const core::ffi::c_void, count: usize) -> isize;
```

---

## 意图
将 `buf` 中最多 `count` 字节写入文件描述符 `fd`。Rust 侧通过 `extern "C"` 导出符号，内部直接调用 Linux `write` 系统调用（`libc::write` 或内联 `syscall!`）。

## 前置条件
- `fd` 是已打开且可写的文件描述符（`RawFd`）
- `buf` 非空，指向至少 `count` 字节的可读内存

## 后置条件
- Case 1 成功: 返回 `Ok(n)`，`0 <= n <= count`，表示实际写入字节数
- Case 2 失败: 返回 `Err(io::Error)`（C ABI 层返回 `-1` 并设置 `errno`）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现直接委托 `syscall_cp(SYS_write, fd, buf, count)`。Rust 中：

```rust
// 方案一：直接使用 libc crate
use libc::{write, ssize_t};

// 方案二：内联 syscall（no_std 环境）
#[inline]
unsafe fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_write, fd, buf, count)
    // aarch64: svc #0 with x8 = SYS_write
}
```

由于 POSIX `write` 本身不要求线程取消点检查（那是 `syscall_cp` 的附加语义），Rust 实现可：
1. 直接调用底层 syscall
2. 或通过 `libc::write` 间接调用

关于取消安全：Rust 的 `async`/`tokio` 生态通常不依赖 `pthread_cancel`，因此取消点语义作为可选特性，不在最小实现中强制。

---

## Rust 安全包装（模块内部）

```rust
use std::os::unix::io::RawFd;
use std::io;

// 安全包装，接受 &[u8] 切片
pub(crate) fn write_bytes(fd: RawFd, data: &[u8]) -> io::Result<usize> {
    let nwritten = unsafe { write_syscall(fd, data.as_ptr(), data.len()) };
    if nwritten < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(nwritten as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  libc::write / syscall!(SYS_write, ...)  // 依赖1: 底层 write 系统调用
  std::os::unix::io::RawFd                 // 依赖2: 文件描述符类型（可选）
Predefined Macros/Crates:
  libc crate                               // 依赖3: 提供 SYS_write 常量及 syscall 函数

[GUARANTEE]
Exported Interface:
  extern "C" fn write(fd: core::ffi::c_int, buf: *const core::ffi::c_void, count: usize) -> isize;
                                 // 本模块保证对外提供与 C ABI 兼容的 write 符号
Internal Interface:
  pub(crate) fn write_bytes(fd: RawFd, data: &[u8]) -> io::Result<usize>;
                                 // 安全 Rust 包装