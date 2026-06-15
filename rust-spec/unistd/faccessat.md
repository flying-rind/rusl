# faccessat — Rust 接口归约

## 原始 C 接口
```c
int faccessat(int fd, const char *filename, int amode, int flag);
```

---

## Rust 外部 ABI 接口

```rust
// 与 C ABI 兼容的底层导出函数
unsafe extern "C" fn faccessat(
    fd: core::ffi::c_int,
    filename: *const core::ffi::c_char,
    amode: core::ffi::c_int,
    flag: core::ffi::c_int,
) -> core::ffi::c_int;
```

---

## 意图
相对于目录文件描述符 `fd` 检查文件的访问权限。支持 `AT_EACCESS` 标志（使用有效而非真实 UID/GID 检查）和 `AT_SYMLINK_NOFOLLOW` 标志（不跟随符号链接）。

当需要 `AT_EACCESS` 且调用者的真实和有效 UID/GID 不同时，musl 通过 `__clone` 创建子进程并在子进程中临时切换身份来执行检查，因为内核空间不支持 `setreuid` 后的真实 UID 访问检查。

## 前置条件
- `fd`: 有效目录文件描述符，或 `AT_FDCWD`（相对于当前工作目录）
- `filename`: 以 NULL 结尾的有效相对/绝对路径
- `amode`: `F_OK`、`R_OK|W_OK|X_OK` 的组合
- `flag`: 0、`AT_EACCESS`（使用有效 ID 检查）或 `AT_SYMLINK_NOFOLLOW` 的组合

## 后置条件
- Case 1 允许访问: 返回 0
- Case 2 错误: 返回 -1，设置 errno

## 不变量
- `AT_EACCESS` 回退路径：信号全阻塞期间，父子进程通过管道通信确保结果的正确传递

## 算法
原 C 实现的核心流程：

```
faccessat(fd, filename, amode, flag):
  if flag:
    ret = __syscall(SYS_faccessat2, fd, filename, amode, flag)  // Linux 5.8+ 原生支持
    if ret != -ENOSYS: return __syscall_ret(ret)

  if flag & ~AT_EACCESS: return __syscall_ret(-EINVAL)

  if !flag || (getuid()==geteuid() && getgid()==getegid()):
    return syscall(SYS_faccessat, fd, filename, amode)

  // getuid != geteuid 的情况：使用子进程进行真实 ID 访问检查
  pipe2(p, O_CLOEXEC)
  __block_all_sigs(&set)
  pid = __clone(checker, stack, 0, &ctx)
  读管道获得子进程结果
  __sys_wait4(pid, &status, __WCLONE, 0)
  __restore_sigs(&set)
  return __syscall_ret(ret)
```

子进程 checker 算法：

```
checker(ctx):
  __syscall(SYS_setregid, __syscall(SYS_getegid), -1)
  __syscall(SYS_setreuid, __syscall(SYS_geteuid), -1)
  ret = __syscall(SYS_faccessat, fd, filename, amode, 0)
  __syscall(SYS_write, pipe_fd, &ret, sizeof ret)
  return 0
```

Rust 中，内部实现需要：
1. 尝试 `SYS_faccessat2`（Linux 5.8+）
2. 回退到 `SYS_faccessat`（若无需特殊处理）
3. 对于 `AT_EACCESS` 回退路径，使用 `clone` 创建子进程，通过管道通信

关于 clone/子进程通信：这属于极其底层的操作，可用 Rust 的 `unsafe` 块直接调用 `clone` 系统调用来实现。管道通信部分使用 `pipe2` + `read`/`write` syscall。

---

## Rust 安全包装（模块内部）

```rust
use core::ffi::CStr;
use std::os::unix::io::RawFd;
use std::io;

// 安全包装
pub(crate) fn faccessat_safe(
    fd: RawFd,
    path: &CStr,
    amode: core::ffi::c_int,
    flag: core::ffi::c_int,
) -> io::Result<()> {
    let ret = unsafe { faccessat_impl(fd, path.as_ptr(), amode, flag) };
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
  SYS_faccessat2 — Linux 5.8+ 系统调用号（原生支持 AT_EACCESS）
  SYS_faccessat — Linux 内核系统调用号
  SYS_clone — 创建子进程系统调用号
  __sys_wait4 / SYS_wait4 — 等待子进程系统调用号
  __block_all_sigs / __restore_sigs — 信号阻塞管理（内部函数，可用 sigprocmask syscall 替代）
  SYS_setregid / SYS_setreuid — 临时切换用户/组 ID 系统调用号
  SYS_pipe2 / SYS_read / SYS_write / SYS_close — 父子进程间通信系统调用号
  getuid / geteuid / getgid / getegid — 用户/组 ID 查询系统调用号
Predefined Macros/Crates:
  无 — 纯系统调用，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  unsafe extern "C" fn faccessat(
      fd: core::ffi::c_int,
      filename: *const core::ffi::c_char,
      amode: core::ffi::c_int,
      flag: core::ffi::c_int,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 faccessat 符号
Internal Interface:
  pub(crate) fn faccessat_safe(
      fd: RawFd,
      path: &CStr,
      amode: core::ffi::c_int,
      flag: core::ffi::c_int,
  ) -> io::Result<()>;
                                 // 安全 Rust 包装
