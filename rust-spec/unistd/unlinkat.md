# unlinkat — Rust 接口归约

## 原始 C 接口
```c
int unlinkat(int fd, const char *path, int flag);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn unlinkat(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    flag: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符 `fd` 删除文件名。支持 `AT_REMOVEDIR` 标志（删除目录而非文件）。

## 前置条件
- `fd`: 目录 fd 或 `AT_FDCWD`
- `path`: 相对或绝对路径
- `flag`: 0 或 `AT_REMOVEDIR`

## 后置条件
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现为纯系统调用封装：`syscall(SYS_unlinkat, fd, path, flag)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_unlinkat(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    flag: core::ffi::c_int,
) -> core::ffi::c_int {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_unlinkat, fd, path, flag)
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn unlinkat_safe(
    fd: RawFd,
    path: &CStr,
    flag: core::ffi::c_int,
) -> io::Result<()> {
    let ret = unsafe { sys_unlinkat(fd, path.as_ptr(), flag) };
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
  SYS_unlinkat — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn unlinkat(
      fd: core::ffi::c_int,
      path: *const core::ffi::c_char,
      flag: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 unlinkat 符号
Internal Interface:
  pub(crate) fn unlinkat_safe(
      fd: RawFd,
      path: &CStr,
      flag: core::ffi::c_int,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
