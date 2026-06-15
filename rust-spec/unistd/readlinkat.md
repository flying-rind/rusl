# readlinkat — Rust 接口归约

## 原始 C 接口
```c
ssize_t readlinkat(int fd, const char *restrict path, char *restrict buf, size_t bufsize);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn readlinkat(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    buf: *mut core::ffi::c_char,
    bufsize: usize,
) -> isize;
```

---

## 意图
相对于目录文件描述符 `fd` 读取符号链接的目标路径。与 `readlink` 相同的语义，但支持相对路径。

## 前置条件
- `fd`: 目录文件描述符或 `AT_FDCWD`
- `path`: 相对或绝对符号链接路径
- `buf` / `bufsize`: 同 readlink

## 后置条件
- Case 1 成功: 返回写入字节数（不含 '\0'），bufsize==0 时返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
- 当 `bufsize == 0` 时，函数使用私有栈缓冲区保护内核调用，然后返回 0

## 算法
原 C 实现的核心流程：

```
readlinkat(fd, path, buf, bufsize):
  if !bufsize: buf = dummy; bufsize = 1
  r = __syscall(SYS_readlinkat, fd, path, buf, bufsize)
  if buf == dummy && r > 0: r = 0
  return __syscall_ret(r)
```

Rust 中：

```rust
// 方案：与 readlink 相同的 bufsize==0 保护逻辑
#[inline]
unsafe fn readlinkat_impl(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    buf: *mut core::ffi::c_char,
    bufsize: usize,
) -> isize {
    let (buf_ptr, size) = if bufsize == 0 {
        let mut dummy: u8 = 0;
        (&raw mut dummy as *mut core::ffi::c_char, 1usize)
    } else {
        (buf, bufsize)
    };
    let r = sys_readlinkat(fd, path, buf_ptr, size);
    if bufsize == 0 && r > 0 { 0 } else { r }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn readlinkat_safe(
    fd: RawFd,
    path: &CStr,
    buf: &mut [u8],
) -> io::Result<usize> {
    let ret = unsafe {
        readlinkat_impl(fd, path.as_ptr(), buf.as_mut_ptr() as *mut core::ffi::c_char, buf.len())
    };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret as usize)
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_readlinkat — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn readlinkat(
      fd: core::ffi::c_int,
      path: *const core::ffi::c_char,
      buf: *mut core::ffi::c_char,
      bufsize: usize,
  ) -> isize;
                                 // 本模块保证对外提供与 C ABI 兼容的 readlinkat 符号
Internal Interface:
  pub(crate) fn readlinkat_safe(
      fd: RawFd,
      path: &CStr,
      buf: &mut [u8],
  ) -> io::Result<usize>;
                                 // 安全 Rust 包装
