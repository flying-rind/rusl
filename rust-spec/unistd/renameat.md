# renameat — Rust 接口归约

## 原始 C 接口
```c
int renameat(int oldfd, const char *old, int newfd, const char *new);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn renameat(
    oldfd: core::ffi::c_int,
    old: *const core::ffi::c_char,
    newfd: core::ffi::c_int,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符原子性地重命名文件/目录。

## 前置条件
- `oldfd`: `old` 的基目录 fd 或 `AT_FDCWD`
- `newfd`: `new` 的基目录 fd 或 `AT_FDCWD`
- `old` / `new`: 相对或绝对路径

## 后置条件
- Case 1 成功: 文件被重命名，返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
renameat(oldfd, old, newfd, new):
  #ifdef SYS_renameat:
    return syscall(SYS_renameat, oldfd, old, newfd, new)
  #else:
    return syscall(SYS_renameat2, oldfd, old, newfd, new, 0)
  #endif
```

Rust 中：

```rust
// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn renameat_impl(
    oldfd: core::ffi::c_int,
    old: *const core::ffi::c_char,
    newfd: core::ffi::c_int,
    new: *const core::ffi::c_char,
) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_renameat(oldfd, old, newfd, new) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_renameat2(oldfd, old, newfd, new, 0) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn renameat_safe(
    oldfd: RawFd,
    old: &CStr,
    newfd: RawFd,
    new: &CStr,
) -> io::Result<()> {
    let ret = unsafe { renameat_impl(oldfd, old.as_ptr(), newfd, new.as_ptr()) };
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
  SYS_renameat — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_renameat2 — Linux 内核系统调用号（回退方案）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn renameat(
      oldfd: core::ffi::c_int,
      old: *const core::ffi::c_char,
      newfd: core::ffi::c_int,
      new: *const core::ffi::c_char,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 renameat 符号
Internal Interface:
  pub(crate) fn renameat_safe(
      oldfd: RawFd,
      old: &CStr,
      newfd: RawFd,
      new: &CStr,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
