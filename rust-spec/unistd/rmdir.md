# rmdir — Rust 接口归约

## 原始 C 接口
```c
int rmdir(const char *path);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn rmdir(path: *const core::ffi::c_char) -> core::ffi::c_int;
```

---

## 意图
删除空目录 `path`。若目录非空（包含 `.` 和 `..` 以外的条目），删除失败。

## 前置条件
- `path`: 存在的空目录路径
- 调用者对目录有写权限

## 后置条件
- Case 1 成功: 目录被删除，返回 0
- Case 2 目录非空: 返回 -1，errno = `ENOTEMPTY`
- Case 3 其他错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
rmdir(path):
  #ifdef SYS_rmdir:
    return syscall(SYS_rmdir, path)
  #else:
    return syscall(SYS_unlinkat, AT_FDCWD, path, AT_REMOVEDIR)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;
const AT_REMOVEDIR: core::ffi::c_int = 0x200;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn rmdir_impl(path: *const core::ffi::c_char) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_rmdir(path) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_unlinkat(AT_FDCWD, path, AT_REMOVEDIR) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn rmdir_safe(path: &CStr) -> io::Result<()> {
    let ret = unsafe { rmdir_impl(path.as_ptr()) };
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
  SYS_rmdir — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_unlinkat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
  AT_REMOVEDIR — 删除目录标志（0x200）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn rmdir(path: *const core::ffi::c_char) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 rmdir 符号
Internal Interface:
  pub(crate) fn rmdir_safe(path: &CStr) -> io::Result<()>;
                                 // 安全 Rust 包装
