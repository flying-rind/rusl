# symlinkat — Rust 接口归约

## 原始 C 接口
```c
int symlinkat(const char *existing, int fd, const char *new);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn symlinkat(
    existing: *const core::ffi::c_char,
    fd: core::ffi::c_int,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符 `fd` 创建符号链接。

## 前置条件
- `existing`: 链接目标路径字符串
- `fd`: 新链接的基目录 fd 或 `AT_FDCWD`
- `new`: 相对或绝对路径名

## 后置条件
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现为纯系统调用封装：`syscall(SYS_symlinkat, existing, fd, new)`。

Rust 中：

```rust
#[inline]
unsafe fn sys_symlinkat(
    existing: *const core::ffi::c_char,
    fd: core::ffi::c_int,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int {
    // arch-specific syscall invocation
    // x86_64: syscall!(SYS_symlinkat, existing, fd, new)
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn symlinkat_safe(
    target: &CStr,
    fd: RawFd,
    linkpath: &CStr,
) -> io::Result<()> {
    let ret = unsafe { sys_symlinkat(target.as_ptr(), fd, linkpath.as_ptr()) };
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
  SYS_symlinkat — Linux 内核系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn symlinkat(
      existing: *const core::ffi::c_char,
      fd: core::ffi::c_int,
      new: *const core::ffi::c_char,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 symlinkat 符号
Internal Interface:
  pub(crate) fn symlinkat_safe(
      target: &CStr,
      fd: RawFd,
      linkpath: &CStr,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
