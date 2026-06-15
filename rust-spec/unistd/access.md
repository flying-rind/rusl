# access — Rust 接口归约

## 原始 C 接口
```c
int access(const char *filename, int amode);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn access(
    filename: *const core::ffi::c_char,
    amode: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图
使用调用进程的**真实**（而非有效）UID/GID 检查对文件 `filename` 的访问权限。与直接尝试 `open()` 不同，access 不会因有效 UID 的特权而错误报告权限。

## 前置条件
- `filename`: 以 NULL 结尾的有效文件路径字符串
- `amode`: `F_OK` (0) 测试存在性，或 `R_OK|W_OK|X_OK` 的按位或组合

## 后置条件
- Case 1 允许访问: 返回 0
- Case 2 拒绝访问或文件不存在: 返回 -1，errno = `EACCES`（权限不足）或 `ENOENT`（文件不存在）
- Case 3 其他错误: 返回 -1，设置 errno（`ENAMETOOLONG`、`ENOTDIR`、`ELOOP`、`EIO` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
access(filename, amode):
  #ifdef SYS_access:
    return syscall(SYS_access, filename, amode)
  #else:
    return syscall(SYS_faccessat, AT_FDCWD, filename, amode, 0)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn access_impl(filename: *const core::ffi::c_char, amode: core::ffi::c_int) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_access(filename, amode) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_faccessat(AT_FDCWD, filename, amode, 0) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 权限检查标志常量
pub const F_OK: core::ffi::c_int = 0;
pub const X_OK: core::ffi::c_int = 1;
pub const W_OK: core::ffi::c_int = 2;
pub const R_OK: core::ffi::c_int = 4;

// 安全包装
pub(crate) fn access_safe(path: &CStr, amode: core::ffi::c_int) -> io::Result<()> {
    let ret = unsafe { access_impl(path.as_ptr(), amode) };
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
  SYS_access — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_faccessat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn access(
      filename: *const core::ffi::c_char,
      amode: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 access 符号
Internal Interface:
  pub(crate) fn access_safe(path: &CStr, amode: core::ffi::c_int) -> io::Result<()>;
                                 // 安全 Rust 包装
  pub const F_OK: core::ffi::c_int;  // 常量
  pub const R_OK: core::ffi::c_int;  // 常量
  pub const W_OK: core::ffi::c_int;  // 常量
  pub const X_OK: core::ffi::c_int;  // 常量
