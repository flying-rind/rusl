# link — Rust 接口归约

## 原始 C 接口
```c
int link(const char *existing, const char *new);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn link(
    existing: *const core::ffi::c_char,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int;
```

---

## 意图
创建文件 `existing` 的新硬链接 `new`。两个路径名指向同一 inode，共享所有数据和元数据，删除任一链接不会影响另一个。

## 前置条件
- `existing`: 已存在文件（不能是目录，除非是 root）
- `new`: 必须不存在（不能覆盖已有文件）
- `existing` 和 `new` 不能在不同文件系统上

## 后置条件
- Case 1 成功: 创建硬链接，返回 0
- Case 2 错误: 返回 -1，设置 errno（`EEXIST`、`EXDEV`、`EPERM`、`ENOENT` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
link(existing, new):
  #ifdef SYS_link:
    return syscall(SYS_link, existing, new)
  #else:
    return syscall(SYS_linkat, AT_FDCWD, existing, AT_FDCWD, new, 0)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn link_impl(
    existing: *const core::ffi::c_char,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_link(existing, new) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_linkat(AT_FDCWD, existing, AT_FDCWD, new, 0) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn link_safe(existing: &CStr, new: &CStr) -> io::Result<()> {
    let ret = unsafe { link_impl(existing.as_ptr(), new.as_ptr()) };
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
  SYS_link — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_linkat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn link(
      existing: *const core::ffi::c_char,
      new: *const core::ffi::c_char,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 link 符号
Internal Interface:
  pub(crate) fn link_safe(existing: &CStr, new: &CStr) -> io::Result<()>;
                                 // 安全 Rust 包装
