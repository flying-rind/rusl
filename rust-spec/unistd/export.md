# unistd 模块 — 对外导出 API 汇总

本文件记录 `src/unistd/` 下所有在 `<unistd.h>` 中声明的、用户程序可直接调用的公开接口，以及对应的 Rust `extern "C"` ABI 设计。内部实现符号（`__` 前缀、`hidden` 可见性等）不在此列，但 rusl 必须同时导出它们以兼容 musl。

---

## 宏常量

| 符号 | 类型 | 值 | 说明 |
|------|------|-----|------|
| `STDIN_FILENO` | `pub const i32` | `0` | 标准输入文件描述符 |
| `STDOUT_FILENO` | `pub const i32` | `1` | 标准输出文件描述符 |
| `STDERR_FILENO` | `pub const i32` | `2` | 标准错误输出文件描述符 |
| `SEEK_SET` | `pub const i32` | `0` | lseek: 从文件起始偏移 |
| `SEEK_CUR` | `pub const i32` | `1` | lseek: 从当前位置偏移 |
| `SEEK_END` | `pub const i32` | `2` | lseek: 从文件末尾偏移 |
| `SEEK_DATA` | `pub const i32` | `3` | lseek: 定位到下一个数据区域 |
| `SEEK_HOLE` | `pub const i32` | `4` | lseek: 定位到下一个空洞区域 |
| `F_OK` | `pub const i32` | `0` | access: 测试文件存在性 |
| `R_OK` | `pub const i32` | `4` | access: 测试读权限 |
| `W_OK` | `pub const i32` | `2` | access: 测试写权限 |
| `X_OK` | `pub const i32` | `1` | access: 测试执行权限 |

在 Rust 中，这些宏常量可以定义在 `unistd` 模块中作为 `pub const` 导出：

```rust
pub const STDIN_FILENO: core::ffi::c_int = 0;
pub const STDOUT_FILENO: core::ffi::c_int = 1;
pub const STDERR_FILENO: core::ffi::c_int = 2;
pub const SEEK_SET: core::ffi::c_int = 0;
pub const SEEK_CUR: core::ffi::c_int = 1;
pub const SEEK_END: core::ffi::c_int = 2;
pub const SEEK_DATA: core::ffi::c_int = 3;
pub const SEEK_HOLE: core::ffi::c_int = 4;
pub const F_OK: core::ffi::c_int = 0;
pub const R_OK: core::ffi::c_int = 4;
pub const W_OK: core::ffi::c_int = 2;
pub const X_OK: core::ffi::c_int = 1;
```

---

## 类型定义

在 Rust 中，这些 C 类型使用 `core::ffi` 的类型别名：

```rust
pub type size_t = usize;
pub type ssize_t = isize;
pub type uid_t = core::ffi::c_uint;
pub type gid_t = core::ffi::c_uint;
pub type off_t = i64;          // 64 位文件偏移
pub type pid_t = core::ffi::c_int;
pub type useconds_t = core::ffi::c_uint;
```

---

## 1. 基本 I/O

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `read` | `extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;` | 从文件描述符读取数据 |
| `write` | `extern "C" fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;` | 向文件描述符写入数据 |
| `close` | `extern "C" fn close(fd: c_int) -> c_int;` | 关闭文件描述符 |
| `dup` | `extern "C" fn dup(fd: c_int) -> c_int;` | 复制文件描述符 |
| `dup2` | `extern "C" fn dup2(old: c_int, new: c_int) -> c_int;` | 复制文件描述符到指定编号 |
| `dup3` | `extern "C" fn dup3(old: c_int, new: c_int, flags: c_int) -> c_int;` | 带标志复制文件描述符（GNU） |
| `lseek` | `extern "C" fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;` | 移动文件读写位置 |
| `pipe` | `extern "C" fn pipe(fd: *mut c_int) -> c_int;` | 创建管道 |
| `pipe2` | `extern "C" fn pipe2(fd: *mut c_int, flags: c_int) -> c_int;` | 带标志创建管道 |
| `pread` | `extern "C" fn pread(fd: c_int, buf: *mut c_void, size: usize, ofs: i64) -> isize;` | 从指定偏移读取（原子操作） |
| `pwrite` | `extern "C" fn pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: i64) -> isize;` | 向指定偏移写入（原子操作） |
| `readv` | `extern "C" fn readv(fd: c_int, iov: *const iovec, count: c_int) -> isize;` | 分散读取 |
| `writev` | `extern "C" fn writev(fd: c_int, iov: *const iovec, count: c_int) -> isize;` | 聚集写入 |
| `preadv` | `extern "C" fn preadv(fd: c_int, iov: *const iovec, count: c_int, ofs: i64) -> isize;` | 从指定偏移分散读取（BSD） |
| `pwritev` | `extern "C" fn pwritev(fd: c_int, iov: *const iovec, count: c_int, ofs: i64) -> isize;` | 向指定偏移聚集写入（GNU） |
| `fsync` | `extern "C" fn fsync(fd: c_int) -> c_int;` | 同步文件数据和元数据到磁盘 |
| `fdatasync` | `extern "C" fn fdatasync(fd: c_int) -> c_int;` | 同步文件数据到磁盘 |
| `ftruncate` | `extern "C" fn ftruncate(fd: c_int, length: i64) -> c_int;` | 截断文件到指定长度 |
| `posix_close` | `extern "C" fn posix_close(fd: c_int, flags: c_int) -> c_int;` | POSIX 关闭文件描述符 |

## 2. 文件系统操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `access` | `extern "C" fn access(filename: *const c_char, amode: c_int) -> c_int;` | 检查文件访问权限 |
| `faccessat` | `extern "C" fn faccessat(fd: c_int, filename: *const c_char, amode: c_int, flag: c_int) -> c_int;` | 相对于目录 fd 检查文件访问权限 |
| `chdir` | `extern "C" fn chdir(path: *const c_char) -> c_int;` | 改变当前工作目录 |
| `fchdir` | `extern "C" fn fchdir(fd: c_int) -> c_int;` | 通过文件描述符改变当前工作目录 |
| `getcwd` | `extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char;` | 获取当前工作目录路径 |
| `chown` | `extern "C" fn chown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int;` | 改变文件所有者 |
| `fchown` | `extern "C" fn fchown(fd: c_int, uid: c_uint, gid: c_uint) -> c_int;` | 通过文件描述符改变文件所有者 |
| `lchown` | `extern "C" fn lchown(path: *const c_char, uid: c_uint, gid: c_uint) -> c_int;` | 改变符号链接自身所有者 |
| `fchownat` | `extern "C" fn fchownat(fd: c_int, path: *const c_char, uid: c_uint, gid: c_uint, flag: c_int) -> c_int;` | 相对于目录 fd 改变文件所有者 |
| `link` | `extern "C" fn link(existing: *const c_char, new: *const c_char) -> c_int;` | 创建硬链接 |
| `linkat` | `extern "C" fn linkat(fd1: c_int, existing: *const c_char, fd2: c_int, new: *const c_char, flag: c_int) -> c_int;` | 相对于目录 fd 创建硬链接 |
| `symlink` | `extern "C" fn symlink(existing: *const c_char, new: *const c_char) -> c_int;` | 创建符号链接 |
| `symlinkat` | `extern "C" fn symlinkat(existing: *const c_char, fd: c_int, new: *const c_char) -> c_int;` | 相对于目录 fd 创建符号链接 |
| `readlink` | `extern "C" fn readlink(path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize;` | 读取符号链接目标路径 |
| `readlinkat` | `extern "C" fn readlinkat(fd: c_int, path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize;` | 相对于目录 fd 读取符号链接目标 |
| `unlink` | `extern "C" fn unlink(path: *const c_char) -> c_int;` | 删除文件名（文件） |
| `unlinkat` | `extern "C" fn unlinkat(fd: c_int, path: *const c_char, flag: c_int) -> c_int;` | 相对于目录 fd 删除文件名 |
| `rmdir` | `extern "C" fn rmdir(path: *const c_char) -> c_int;` | 删除空目录 |
| `renameat` | `extern "C" fn renameat(oldfd: c_int, old: *const c_char, newfd: c_int, new: *const c_char) -> c_int;` | 相对于目录 fd 重命名文件 |
| `truncate` | `extern "C" fn truncate(path: *const c_char, length: i64) -> c_int;` | 通过路径截断文件 |
| `sync` | `extern "C" fn sync();` | 同步所有文件系统缓冲区（BSD/XOPEN） |
| `acct` | `extern "C" fn acct(filename: *const c_char) -> c_int;` | 启用/禁用进程记账（GNU） |

## 3. 进程控制

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `_exit` | `extern "C" fn _exit(status: c_int) -> !;` | 立即终止进程（无清理） |
| `alarm` | `extern "C" fn alarm(seconds: c_uint) -> c_uint;` | 设置 SIGALRM 定时器 |
| `sleep` | `extern "C" fn sleep(seconds: c_uint) -> c_uint;` | 睡眠指定秒数 |
| `pause` | `extern "C" fn pause() -> c_int;` | 等待信号 |
| `getpid` | `extern "C" fn getpid() -> c_int;` | 获取当前进程 ID |
| `getppid` | `extern "C" fn getppid() -> c_int;` | 获取父进程 ID |
| `getpgrp` | `extern "C" fn getpgrp() -> c_int;` | 获取进程组 ID |
| `getpgid` | `extern "C" fn getpgid(pid: c_int) -> c_int;` | 获取指定进程的进程组 ID |
| `setpgid` | `extern "C" fn setpgid(pid: c_int, pgid: c_int) -> c_int;` | 设置进程组 ID |
| `setsid` | `extern "C" fn setsid() -> c_int;` | 创建新会话 |
| `getsid` | `extern "C" fn getsid(pid: c_int) -> c_int;` | 获取指定进程的会话 ID |
| `setpgrp` | `extern "C" fn setpgrp() -> c_int;` | 设置进程组（BSD/XOPEN，等价 setpgid(0,0)） |
| `nice` | `extern "C" fn nice(inc: c_int) -> c_int;` | 调整进程调度优先级（BSD/XOPEN） |

## 4. 用户标识和组

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `getuid` | `extern "C" fn getuid() -> c_uint;` | 获取真实用户 ID |
| `geteuid` | `extern "C" fn geteuid() -> c_uint;` | 获取有效用户 ID |
| `getgid` | `extern "C" fn getgid() -> c_uint;` | 获取真实组 ID |
| `getegid` | `extern "C" fn getegid() -> c_uint;` | 获取有效组 ID |
| `getgroups` | `extern "C" fn getgroups(count: c_int, list: *mut c_uint) -> c_int;` | 获取附加组列表 |
| `setuid` | `extern "C" fn setuid(uid: c_uint) -> c_int;` | 设置用户 ID |
| `seteuid` | `extern "C" fn seteuid(euid: c_uint) -> c_int;` | 设置有效用户 ID |
| `setgid` | `extern "C" fn setgid(gid: c_uint) -> c_int;` | 设置组 ID |
| `setegid` | `extern "C" fn setegid(egid: c_uint) -> c_int;` | 设置有效组 ID |
| `setreuid` | `extern "C" fn setreuid(ruid: c_uint, euid: c_uint) -> c_int;` | 设置真实和有效用户 ID（BSD/XOPEN） |
| `setregid` | `extern "C" fn setregid(rgid: c_uint, egid: c_uint) -> c_int;` | 设置真实和有效组 ID（BSD/XOPEN） |

## 5. 用户标识和组 — GNU 扩展

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `setresuid` | `extern "C" fn setresuid(ruid: c_uint, euid: c_uint, suid: c_uint) -> c_int;` | 设置真实/有效/保存用户 ID（GNU） |
| `setresgid` | `extern "C" fn setresgid(rgid: c_uint, egid: c_uint, sgid: c_uint) -> c_int;` | 设置真实/有效/保存组 ID（GNU） |

## 6. 终端操作

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `ttyname` | `extern "C" fn ttyname(fd: c_int) -> *mut c_char;` | 获取终端设备名称 |
| `ttyname_r` | `extern "C" fn ttyname_r(fd: c_int, name: *mut c_char, size: usize) -> c_int;` | 获取终端设备名称（线程安全版） |
| `isatty` | `extern "C" fn isatty(fd: c_int) -> c_int;` | 测试文件描述符是否关联终端 |
| `tcgetpgrp` | `extern "C" fn tcgetpgrp(fd: c_int) -> c_int;` | 获取终端前台进程组 |
| `tcsetpgrp` | `extern "C" fn tcsetpgrp(fd: c_int, pgrp: c_int) -> c_int;` | 设置终端前台进程组 |
| `ctermid` | `extern "C" fn ctermid(s: *mut c_char) -> *mut c_char;` | 获取控制终端路径名 |

## 7. 其他系统信息

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `gethostname` | `extern "C" fn gethostname(name: *mut c_char, len: usize) -> c_int;` | 获取主机名 |
| `getlogin` | `extern "C" fn getlogin() -> *mut c_char;` | 获取登录用户名 |
| `getlogin_r` | `extern "C" fn getlogin_r(name: *mut c_char, size: usize) -> c_int;` | 获取登录用户名（线程安全版） |

## 8. BSD/GNU 扩展 — 时间

| 符号 | Rust extern "C" 签名 | 说明 |
|------|---------------------|------|
| `usleep` | `extern "C" fn usleep(useconds: c_uint) -> c_int;` | 睡眠指定微秒数 |
| `ualarm` | `extern "C" fn ualarm(value: c_uint, interval: c_uint) -> c_uint;` | 设置微秒级间隔定时器 |

---

## musl `__` 前缀内部符号（rusl 必须同时导出）

musl 中 `__xxx` 是主实现，`xxx` 是其弱别名。rusl 必须同时提供两者：

| 内部符号 | Rust extern "C" 签名 | 公开别名 |
|---------|---------------------|---------|
| `__dup3` | `extern "C" fn __dup3(old: c_int, new: c_int, flags: c_int) -> c_int;` | `dup3` |
| `__setxid` | `extern "C" fn __setxid(nr: c_int, id: c_int, eid: c_int, sid: c_int) -> c_int;` | (内部符号，无公开别名) |

---

## 排除说明

以下类别的符号不需要在 Rust 外部 ABI 中导出：

- 所有 `static` 内部函数（已重构为 Rust 私有函数）
- 内部静态变量（已重构为 Rust `static` 或 `&'static str`）
- 架构特定实现（作为 `cfg(target_arch = "...")` 的条件编译模块）
- `_LARGEFILE64_SOURCE` 兼容宏（Rust 中不需要，可直接使用标准 API）

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  各子模块的 extern "C" 导出函数定义
Predefined Macros/Crates:
  core::ffi — Rust 核心库的 FFI 类型 (c_int, c_uint, c_char, c_void, isize, usize)

[GUARANTEE]
Exported Interface:
  上述所有符号均须以 extern "C" 导出，保证与 C ABI 完全兼容
  musl __ 前缀内部符号必须同时导出（作为独立符号，非弱别名）
