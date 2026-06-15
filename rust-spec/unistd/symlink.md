# symlink — Rust 接口归约

## 原始 C 接口
```c
int symlink(const char *existing, const char *new);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn symlink(
    existing: *const core::ffi::c_char,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int;
```

---

## 意图
创建符号链接 `new` 指向目标 `existing`（目标不需要存在）。符号链接是包含目标路径字符串的特殊文件类型。

## 前置条件
- `existing`: 链接目标路径字符串（可为任意字符串，甚至不存在的路径）
- `new`: 符号链接的路径名（必须不存在）

## 后置条件
- Case 1 成功: 创建符号链接，返回 0
- Case 2 错误: 返回 -1，设置 errno（`EEXIST`、`ENOENT`、`EACCES` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
symlink(existing, new):
  #ifdef SYS_symlink:
    return syscall(SYS_symlink, existing, new)
  #else:
    return syscall(SYS_symlinkat, existing, AT_FDCWD, new)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn symlink_impl(
    existing: *const core::ffi::c_char,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_symlink(existing, new) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_symlinkat(existing, AT_FDCWD, new) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn symlink_safe(target: &CStr, linkpath: &CStr) -> io::Result<()> {
    let ret = unsafe { symlink_impl(target.as_ptr(), linkpath.as_ptr()) };
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
  SYS_symlink — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_symlinkat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn symlink(
      existing: *const core::ffi::c_char,
      new: *const core::ffi::c_char,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 symlink 符号
Internal Interface:
  pub(crate) fn symlink_safe(target: &CStr, linkpath: &CStr) -> io::Result<()>;
                                 // 安全 Rust 包装
