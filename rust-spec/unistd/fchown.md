# fchown — Rust 接口归约

## 原始 C 接口
```c
int fchown(int fd, uid_t uid, gid_t gid);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn fchown(
    fd: core::ffi::c_int,
    uid: core::ffi::c_uint,
    gid: core::ffi::c_uint,
) -> core::ffi::c_int;
```

---

## 意图
通过文件描述符 `fd` 改变文件的所有者和/或所属组。实现了与 `fchdir` 类似的 /proc 回退逻辑：当内核直接拒绝 `fchown`（`EBADF`）但 `fd` 实际有效时，通过 `/proc/self/fd/<fd>` 路径名执行 `chown`。

## 前置条件
- `fd`: 有效的已打开文件描述符
- `uid` / `gid`: 新所有者/组（-1 保持不变）

## 后置条件
- Case 1 成功: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
- 当 `fd` 有效但 `fchown` 返回 `EBADF` 时，总是尝试通过 `/proc/self/fd/<fd>` 回退路径

## 算法
原 C 实现的核心流程：

```
fchown(fd, uid, gid):
  ret = __syscall(SYS_fchown, fd, uid, gid)
  if ret != -EBADF: return __syscall_ret(ret)

  if __syscall(SYS_fcntl, fd, F_GETFD) < 0: return __syscall_ret(ret)

  __procfdname(buf, fd)
  #ifdef SYS_chown:
    return syscall(SYS_chown, buf, uid, gid)
  #else:
    return syscall(SYS_fchownat, AT_FDCWD, buf, uid, gid, 0)
  #endif
```

Rust 中：

```rust
const AT_FDCWD: core::ffi::c_int = -100;

// 方案：直接封装内核系统调用，内部实现 fcntl 检查和 /proc 回退
#[inline]
unsafe fn sys_fchown(fd: core::ffi::c_int, uid: u32, gid: u32) -> core::ffi::c_int { /* ... */ }
unsafe fn sys_fcntl(fd: core::ffi::c_int, cmd: core::ffi::c_int) -> core::ffi::c_int { /* ... */ }
fn procfdname(buf: &mut [u8; 40], fd: core::ffi::c_int) -> &[u8] { /* ... */ }
```

---

## Rust 安全包装（模块内部）

```rust
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn fchown_safe(fd: RawFd, uid: u32, gid: u32) -> io::Result<()> {
    let ret = unsafe { fchown_impl(fd, uid, gid) };
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
  SYS_fchown — Linux 内核系统调用号
  SYS_fcntl + F_GETFD — 检查 fd 有效性
  SYS_chown / SYS_fchownat — 回退路径系统调用号
  __procfdname — 构造 /proc/self/fd/ 路径（内部函数）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn fchown(
      fd: core::ffi::c_int,
      uid: core::ffi::c_uint,
      gid: core::ffi::c_uint,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 fchown 符号
Internal Interface:
  pub(crate) fn fchown_safe(fd: RawFd, uid: u32, gid: u32) -> io::Result<()>;
                                 // 安全 Rust 包装
