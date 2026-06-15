# lchown — Rust 接口归约

## 原始 C 接口
```c
int lchown(const char *path, uid_t uid, gid_t gid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn lchown(
    path: *const core::ffi::c_char,
    uid: core::ffi::c_uint,
    gid: core::ffi::c_uint,
) -> core::ffi::c_int;
```

---

## 意图
改变符号链接自身（而非其目标）的所有者和/或所属组。与 `chown` 不同，`lchown` 不跟随符号链接。

## 前置条件
- `path`: 以 NULL 结尾的有效路径
- `uid` / `gid`: 新所有者/组（-1 保持不变）

## 后置条件
同 `chown`，但操作对象是符号链接自身：
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
lchown(path, uid, gid):
  #ifdef SYS_lchown:
    return syscall(SYS_lchown, path, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, path, uid, gid, AT_SYMLINK_NOFOLLOW)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;
const AT_SYMLINK_NOFOLLOW: core::ffi::c_int = 0x100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn lchown_impl(path: *const core::ffi::c_char, uid: u32, gid: u32) -> core::ffi::c_int {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    { sys_lchown(path, uid, gid) }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    { sys_fchownat(AT_FDCWD, path, uid, gid, AT_SYMLINK_NOFOLLOW) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn lchown_safe(path: &CStr, uid: u32, gid: u32) -> io::Result<()> {
    let ret = unsafe { lchown_impl(path.as_ptr(), uid, gid) };
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
  SYS_lchown — Linux 内核系统调用号（部分架构如 x86/x86_64）
  SYS_fchownat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
  AT_SYMLINK_NOFOLLOW — 不跟随符号链接标志（0x100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn lchown(
      path: *const core::ffi::c_char,
      uid: core::ffi::c_uint,
      gid: core::ffi::c_uint,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 lchown 符号
Internal Interface:
  pub(crate) fn lchown_safe(path: &CStr, uid: u32, gid: u32) -> io::Result<()>;
                                 // 安全 Rust 包装
