# fchdir — Rust 接口归约

## 原始 C 接口
```c
int fchdir(int fd);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn fchdir(fd: core::ffi::c_int) -> core::ffi::c_int;
```

---

## 意图
将调用进程的当前工作目录改为文件描述符 `fd` 所引用的目录。musl 实现了特殊的回退逻辑：当内核直接拒绝 `fchdir`（`EBADF`）但 `fd` 实际有效时（某些文件系统/proc 上的目录可能不支持 fchdir），通过 `/proc/self/fd/<fd>` 路径名执行 `chdir`。

## 前置条件
- `fd`: 有效的已打开目录文件描述符

## 后置条件
- Case 1 成功: 当前工作目录变更，返回 0
- Case 2 fd 无效: 返回 -1，errno = `EBADF`
- Case 3 其他错误: 返回 -1，errno 设置（如 `EACCES`、`ENOENT` 等）

## 不变量
- 当 `fd` 有效但 `fchdir` 返回 `EBADF` 时，总是尝试通过 `/proc/self/fd/<fd>` 回退路径

## 算法
原 C 实现的核心流程：

```
fchdir(fd):
  ret = __syscall(SYS_fchdir, fd)
  if ret != -EBADF: return __syscall_ret(ret)

  if __syscall(SYS_fcntl, fd, F_GETFD) < 0:
    return __syscall_ret(ret)  // fd 确实无效

  // fd 有效但 fchdir 返回 EBADF → 通过 /proc 回退
  __procfdname(buf, fd)        // 构造 "/proc/self/fd/<fd>"
  return syscall(SYS_chdir, buf)
```

Rust 中：

```rust
// 方案：直接封装内核系统调用，内部实现 fcntl 检查和 /proc 回退
#[inline]
unsafe fn sys_fchdir(fd: core::ffi::c_int) -> core::ffi::c_int { /* ... */ }
unsafe fn sys_fcntl(fd: core::ffi::c_int, cmd: core::ffi::c_int) -> core::ffi::c_int { /* ... */ }
unsafe fn sys_chdir(path: *const core::ffi::c_char) -> core::ffi::c_int { /* ... */ }
fn procfdname(buf: &mut [u8; 40], fd: core::ffi::c_int) -> &[u8] { /* ... */ }
```

---

## Rust 安全包装（模块内部）

```rust
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn fchdir_safe(fd: RawFd) -> io::Result<()> {
    let ret = unsafe { fchdir_impl(fd) };
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
  SYS_fchdir — Linux 内核系统调用号
  SYS_fcntl + F_GETFD — 检查 fd 有效性
  SYS_chdir — 回退路径系统调用号
  __procfdname — 构造 /proc/self/fd/ 路径（内部函数，可用 Rust 安全实现）
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn fchdir(fd: core::ffi::c_int) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 fchdir 符号
Internal Interface:
  pub(crate) fn fchdir_safe(fd: RawFd) -> io::Result<()>;
                                 // 安全 Rust 包装
  fn procfdname(buf: &mut [u8; 40], fd: core::ffi::c_int) -> &[u8];
                                 // 构造 /proc/self/fd/<fd> 路径（内部）
