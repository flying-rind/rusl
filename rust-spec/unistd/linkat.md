# linkat — Rust 接口归约

## 原始 C 接口
```c
int linkat(int fd1, const char *existing, int fd2, const char *new, int flag);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn linkat(
    fd1: core::ffi::c_int,
    existing: *const core::ffi::c_char,
    fd2: core::ffi::c_int,
    new: *const core::ffi::c_char,
    flag: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符创建硬链接。支持 `AT_SYMLINK_FOLLOW` 标志。

## 前置条件
- `fd1`: `existing` 的基目录 fd 或 `AT_FDCWD`
- `fd2`: `new` 的基目录 fd 或 `AT_FDCWD`
- `existing` / `new`: 相对或绝对路径
- `flag`: 0 或 `AT_SYMLINK_FOLLOW`

## 后置条件
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现为纯系统调用封装：`syscall(SYS_linkat, fd1, existing, fd2, new, flag)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_linkat(
    fd1: core::ffi::c_int,
    existing: *const core::ffi::c_char,
    fd2: core::ffi::c_int,
    new: *const core::ffi::c_char,
    flag: core::ffi::c_int,
) -> core::ffi::c_int {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_linkat, fd1, existing, fd2, new, flag)
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn linkat_safe(
    fd1: RawFd,
    existing: &CStr,
    fd2: RawFd,
    new: &CStr,
    flag: core::ffi::c_int,
) -> io::Result<()> {
    let ret = unsafe { sys_linkat(fd1, existing.as_ptr(), fd2, new.as_ptr(), flag) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  SYS_linkat — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn linkat(
      fd1: core::ffi::c_int,
      existing: *const core::ffi::c_char,
      fd2: core::ffi::c_int,
      new: *const core::ffi::c_char,
      flag: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 linkat 符号
Internal Interface:
  pub(crate) fn linkat_safe(
      fd1: RawFd,
      existing: &CStr,
      fd2: RawFd,
      new: &CStr,
      flag: core::ffi::c_int,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
