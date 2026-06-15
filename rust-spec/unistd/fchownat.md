# fchownat — Rust 接口归约

## 原始 C 接口
```c
int fchownat(int fd, const char *path, uid_t uid, gid_t gid, int flag);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn fchownat(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    uid: core::ffi::c_uint,
    gid: core::ffi::c_uint,
    flag: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符 `fd` 改变文件的所有者和/或所属组。支持 `AT_SYMLINK_NOFOLLOW` 标志（不跟随符号链接）。

## 前置条件
- `fd`: 目录文件描述符或 `AT_FDCWD`
- `path`: 相对或绝对路径
- `uid` / `gid`: 新所有者/组（-1 保持不变）
- `flag`: 0 或 `AT_SYMLINK_NOFOLLOW`

## 后置条件
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现为纯系统调用封装：`syscall(SYS_fchownat, fd, path, uid, gid, flag)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_fchownat(
    fd: core::ffi::c_int,
    path: *const core::ffi::c_char,
    uid: u32,
    gid: u32,
    flag: core::ffi::c_int,
) -> core::ffi::c_int {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_fchownat, fd, path, uid, gid, flag)
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn fchownat_safe(
    fd: RawFd,
    path: &CStr,
    uid: u32,
    gid: u32,
    flag: core::ffi::c_int,
) -> io::Result<()> {
    let ret = unsafe { sys_fchownat(fd, path.as_ptr(), uid, gid, flag) };
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
  SYS_fchownat — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn fchownat(
      fd: core::ffi::c_int,
      path: *const core::ffi::c_char,
      uid: core::ffi::c_uint,
      gid: core::ffi::c_uint,
      flag: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 fchownat 符号
Internal Interface:
  pub(crate) fn fchownat_safe(
      fd: RawFd,
      path: &CStr,
      uid: u32,
      gid: u32,
      flag: core::ffi::c_int,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
