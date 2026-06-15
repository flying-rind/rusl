# unlink — Rust 接口归约

## 原始 C 接口
```c
int unlink(const char *path);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn unlink(path: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
删除文件系统中 `path` 指定的文件名（目录条目）。若该文件名是文件的最后一个硬链接且没有进程打开该文件，文件数据被删除。若 `path` 是符号链接，删除链接自身而非目标。

不能用于删除目录（使用 `rmdir` 或 `unlinkat(..., AT_REMOVEDIR)`）。

## 前置条件
- `path`: 以 NULL 结尾的有效文件路径（不能是目录）

## 后置条件
- Case 1 成功: 文件名被删除，返回 0
- Case 2 错误: 返回 -1，errno = `EPERM`（目录）、`ENOENT`、`EACCES`、`EBUSY` 等

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
unlink(path):
  #ifdef SYS_unlink:
    return syscall(SYS_unlink, path)
  #else:
    return syscall(SYS_unlinkat, AT_FDCWD, path, 0)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn unlink_impl(path: *const core::ffi::c_char) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_unlink(path) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_unlinkat(AT_FDCWD, path, 0) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn unlink_safe(path: &CStr) -> io::Result<()> {
    let ret = unsafe { unlink_impl(path.as_ptr()) };
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
  SYS_unlink — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_unlinkat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn unlink(path: *const core::ffi::c_char) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 unlink 符号
Internal Interface:
  pub(crate) fn unlink_safe(path: &CStr) -> io::Result<()>;
                                 // 安全 Rust 包装
