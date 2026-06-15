# chown — Rust 接口归约

## 原始 C 接口
```c
int chown(const char *path, uid_t uid, gid_t gid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn chown(
    path: *const core::ffi::c_char,
    uid: core::ffi::c_uint,
    gid: core::ffi::c_uint,
) -> core::ffi::c_int;
```

---

## 意图
将文件 `path` 的所有者和/或所属组改为 `uid` 和 `gid`。若 `uid` 或 `gid` 为 -1（即 `(uid_t)-1`），对应属性保持不变。

## 前置条件
- `path`: 以 NULL 结尾的有效文件路径
- `uid`: 新所有者 UID（-1 保持不变）或 `gid`: 新组 GID（-1 保持不变）

## 后置条件
- Case 1 成功: 文件所有者/组变更，返回 0
- Case 2 错误: 返回 -1，设置 errno（`EPERM`、`ENOENT`、`EACCES` 等）

## 不变量
无。本函数不持有任何内部状态。

## 算法
原 C 实现使用条件编译选择系统调用：

```
chown(path, uid, gid):
  #ifdef SYS_chown:
    return syscall(SYS_chown, path, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, path, uid, gid, 0)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：cfg 条件编译选择系统调用
#[inline]
unsafe fn chown_impl(path: *const core::ffi::c_char, uid: u32, gid: u32) -> core::ffi::c_int {
    #[cfg(target_arch = "x86_64")]
    { sys_chown(path, uid, gid) }
    #[cfg(not(target_arch = "x86_64"))]
    { sys_fchownat(AT_FDCWD, path, uid, gid, 0) }
}
```

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::io;

// 安全包装
pub(crate) fn chown_safe(path: &CStr, uid: u32, gid: u32) -> io::Result<()> {
    let ret = unsafe { chown_impl(path.as_ptr(), uid, gid) };
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
  SYS_chown — Linux 内核系统调用号（部分架构如 x86_64）
  SYS_fchownat — Linux 内核系统调用号（回退方案）
  AT_FDCWD — 相对于当前工作目录的特殊 fd 值（-100）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn chown(
      path: *const core::ffi::c_char,
      uid: core::ffi::c_uint,
      gid: core::ffi::c_uint,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 chown 符号
Internal Interface:
  pub(crate) fn chown_safe(path: &CStr, uid: u32, gid: u32) -> io::Result<()>;
                                 // 安全 Rust 包装
