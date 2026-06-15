# unistd 模块 — 外部依赖导入

本文件记录 `src/unistd/` 下所有函数使用到的来自外部模块的 Rust 接口依赖。

---

## 系统调用接口（来自内核 / rusl syscall 模块）

rusl 中使用统一的 `syscall` 模块封装内核系统调用：

| 内核接口 | rusl 内部调用方式 | 说明 |
|---------|-----------------|------|
| `SYS_read` | `syscall::sys_read(fd, buf, count)` | 内核文件读取系统调用 |
| `SYS_write` | `syscall::sys_write(fd, buf, count)` | 内核文件写入系统调用 |
| `SYS_close` | `syscall::sys_close(fd)` | 内核关闭文件描述符系统调用 |
| `SYS_dup` | `syscall::sys_dup(fd)` | 内核复制文件描述符系统调用 |
| `SYS_dup2` | `syscall::sys_dup2(old, new)` | 内核复制文件描述符到指定编号 |
| `SYS_dup3` | `syscall::sys_dup3(old, new, flags)` | 内核带标志复制文件描述符 |
| `SYS_lseek` | `syscall::sys_lseek(fd, offset, whence)` | 内核文件定位系统调用 |
| `SYS_pipe` | `syscall::sys_pipe(fd)` | 内核管道创建系统调用 |
| `SYS_pipe2` | `syscall::sys_pipe2(fd, flags)` | 内核带标志管道创建系统调用 |
| `SYS_pread` | `syscall::sys_pread(fd, buf, size, ofs)` | 内核指定偏移读取系统调用 |
| `SYS_pwrite` | `syscall::sys_pwrite(fd, buf, size, ofs)` | 内核指定偏移写入系统调用 |
| `SYS_readv` | `syscall::sys_readv(fd, iov, count)` | 内核分散读取系统调用 |
| `SYS_writev` | `syscall::sys_writev(fd, iov, count)` | 内核聚集写入系统调用 |
| `SYS_fsync` | `syscall::sys_fsync(fd)` | 内核同步文件元数据系统调用 |
| `SYS_fdatasync` | `syscall::sys_fdatasync(fd)` | 内核同步文件数据系统调用 |
| `SYS_ftruncate` | `syscall::sys_ftruncate(fd, length)` | 内核文件截断系统调用 |
| `SYS_truncate` | `syscall::sys_truncate(path, length)` | 内核路径文件截断系统调用 |
| `SYS_access` | `syscall::sys_access(filename, amode)` | 内核文件访问检查系统调用 |
| `SYS_faccessat` | `syscall::sys_faccessat(fd, filename, amode, flag)` | 内核相对路径访问检查系统调用 |
| `SYS_chdir` | `syscall::sys_chdir(path)` | 内核改变工作目录系统调用 |
| `SYS_fchdir` | `syscall::sys_fchdir(fd)` | 内核描述符改变工作目录系统调用 |
| `SYS_getcwd` | `syscall::sys_getcwd(buf, size)` | 内核获取工作目录系统调用 |
| `SYS_chown` | `syscall::sys_chown(path, uid, gid)` | 内核改变所有者系统调用 |
| `SYS_fchown` | `syscall::sys_fchown(fd, uid, gid)` | 内核描述符改变所有者系统调用 |
| `SYS_lchown` | `syscall::sys_lchown(path, uid, gid)` | 内核符号链接改变所有者系统调用 |
| `SYS_fchownat` | `syscall::sys_fchownat(fd, path, uid, gid, flag)` | 内核相对路径改变所有者系统调用 |
| `SYS_link` | `syscall::sys_link(existing, new)` | 内核创建硬链接系统调用 |
| `SYS_linkat` | `syscall::sys_linkat(fd1, existing, fd2, new, flag)` | 内核相对路径创建硬链接系统调用 |
| `SYS_symlink` | `syscall::sys_symlink(existing, new)` | 内核创建符号链接系统调用 |
| `SYS_symlinkat` | `syscall::sys_symlinkat(existing, fd, new)` | 内核相对路径创建符号链接系统调用 |
| `SYS_readlink` | `syscall::sys_readlink(path, buf, bufsize)` | 内核读取符号链接系统调用 |
| `SYS_readlinkat` | `syscall::sys_readlinkat(fd, path, buf, bufsize)` | 内核相对路径读取符号链接系统调用 |
| `SYS_unlink` | `syscall::sys_unlink(path)` | 内核删除文件系统调用 |
| `SYS_unlinkat` | `syscall::sys_unlinkat(fd, path, flag)` | 内核相对路径删除文件系统调用 |
| `SYS_rmdir` | `syscall::sys_rmdir(path)` | 内核删除目录系统调用 |
| `SYS_renameat` | `syscall::sys_renameat(oldfd, old, newfd, new)` | 内核相对路径重命名系统调用 |
| `SYS_getpid` | `syscall::sys_getpid()` | 内核获取进程 ID 系统调用 |
| `SYS_getppid` | `syscall::sys_getppid()` | 内核获取父进程 ID 系统调用 |
| `SYS_getpgid` | `syscall::sys_getpgid(pid)` | 内核获取进程组 ID 系统调用 |
| `SYS_setpgid` | `syscall::sys_setpgid(pid, pgid)` | 内核设置进程组 ID 系统调用 |
| `SYS_setsid` | `syscall::sys_setsid()` | 内核创建新会话系统调用 |
| `SYS_getsid` | `syscall::sys_getsid(pid)` | 内核获取会话 ID 系统调用 |
| `SYS_getuid` | `syscall::sys_getuid()` | 内核获取真实用户 ID 系统调用 |
| `SYS_geteuid` | `syscall::sys_geteuid()` | 内核获取有效用户 ID 系统调用 |
| `SYS_getgid` | `syscall::sys_getgid()` | 内核获取真实组 ID 系统调用 |
| `SYS_getegid` | `syscall::sys_getegid()` | 内核获取有效组 ID 系统调用 |
| `SYS_getgroups` | `syscall::sys_getgroups(count, list)` | 内核获取附加组列表系统调用 |
| `SYS_setuid` | `syscall::sys_setuid(uid, ...)` (通过 `__setxid`) | 内核设置用户 ID 系统调用 |
| `SYS_setgid` | `syscall::sys_setgid(gid, ...)` (通过 `__setxid`) | 内核设置组 ID 系统调用 |
| `SYS_setreuid` | `syscall::sys_setreuid(ruid, euid, ...)` (通过 `__setxid`) | 内核设置真实/有效用户 ID |
| `SYS_setregid` | `syscall::sys_setregid(rgid, egid, ...)` (通过 `__setxid`) | 内核设置真实/有效组 ID |
| `SYS_setresuid` | `syscall::sys_setresuid(ruid, euid, suid)` (通过 `__setxid`) | 内核设置真实/有效/保存用户 ID |
| `SYS_setresgid` | `syscall::sys_setresgid(rgid, egid, sgid)` (通过 `__setxid`) | 内核设置真实/有效/保存组 ID |
| `SYS_ioctl` | `syscall::sys_ioctl(fd, req, ...)` | 内核设备控制操作 |
| `SYS_fcntl` | `syscall::sys_fcntl(fd, cmd, ...)` | 内核文件描述符控制 |
| `SYS_sync` | `syscall::sys_sync()` | 内核同步所有文件系统缓冲区 |
| `SYS_acct` | `syscall::sys_acct(filename)` | 内核进程记账系统调用 |
| `SYS_exit` | `syscall::sys_exit(code)` | 内核进程退出系统调用 |
| `SYS_kill` | `syscall::sys_kill(pid, sig)` | 内核发送信号系统调用 |

---

## 结构体与类型

| 类型 | Rust 定义位置 | 说明 |
|------|-------------|------|
| `struct iovec` | `rusl::sys::iovec` | 散布/聚集 I/O 向量 |
| `struct utsname` | `rusl::sys::utsname` | 系统名称信息结构（含 nodename 字段 65 字节） |
| `struct winsize` | `rusl::sys::winsize` | 终端窗口大小结构 |
| `struct stat` | `rusl::sys::stat` | 文件状态结构 |
| `sigset_t` | `rusl::signal::SigSet` | 信号集类型 |
| `size_t` | `usize` | 标准大小类型 |
| `ssize_t` | `isize` | 有符号大小类型 |
| `pid_t` | `core::ffi::c_int` (i32) | 进程 ID 类型 |
| `uid_t` | `core::ffi::c_uint` (u32) | 用户 ID 类型 |
| `gid_t` | `core::ffi::c_uint` (u32) | 组 ID 类型 |
| `off_t` | `i64` | 文件偏移类型（64 位） |

---

## 跨模块内部依赖（rusl 内部）

| 接口 | 定义位置 | 说明 |
|------|----------|------|
| `syscall::syscall_ret(r)` | `rusl::syscall` 模块 | 内核返回值到 libc 错误码转换 |
| `syscall::syscall(nr, ...)` | `rusl::syscall` 模块 | 原始系统调用封装 |
| `syscall::syscall_cp(nr, ...)` | `rusl::syscall` 模块 | 可被信号取消的系统调用封装 |
| `uname(buf)` | `rusl::linux::uname` 模块 | 获取系统名称信息 |
| `readlink(path, buf, size)` | `rusl::unistd::readlink` 模块 | 读取符号链接目标 |
| `stat(path, buf)` | `rusl::stat::stat` 模块 | 获取文件状态 |
| `fstat(fd, buf)` | `rusl::stat::fstat` 模块 | 通过文件描述符获取文件状态 |
| `ioctl(fd, req, ...)` | `rusl::misc::ioctl` 模块 | 设备控制操作 |
| `procfdname(buf, fd)` | `rusl::internal::procfdname` 模块 | 构造 `/proc/self/fd/<fd>` 路径字符串 |
| `block_all_signals(set)` | `rusl::signal::block` 模块 | 阻塞所有信号并保存原信号掩码 |
| `synccall(func, ctx)` | `rusl::internal::synccall` 模块 | 在所有线程上同步执行函数 |
| `getenv(name)` | `rusl::env::getenv` 模块 | 获取环境变量值 |
| `strlen(s)` | `rusl::string::strlen` 模块 | 计算 C 字符串长度 |
| `strcpy(dst, src)` | `rusl::string::strcpy` 模块 | C 字符串拷贝 |

---

## 宏与常量

| 宏/常量 | Rust 定义 | 说明 |
|----------|----------|------|
| `TIOCGWINSZ` | `pub const TIOCGWINSZ: i32 = 0x5413;` | 获取终端窗口大小的 ioctl 请求 |
| `TIOCGPGRP` | `pub const TIOCGPGRP: i32 = 0x540F;` | 获取前台进程组的 ioctl 请求 |
| `TIOCSPGRP` | `pub const TIOCSPGRP: i32 = 0x5410;` | 设置前台进程组的 ioctl 请求 |
| `TTY_NAME_MAX` | `pub const TTY_NAME_MAX: usize = 32;` | 终端名称最大长度 |
| `SIGKILL` | `pub const SIGKILL: i32 = 9;` | 不可捕获/忽略的终止信号 |
| `ENOSYS` | `pub const ENOSYS: i32 = ...;` | 系统调用未实现 |
| `EINVAL` | `pub const EINVAL: i32 = ...;` | 无效参数 |
| `EBADF` | `pub const EBADF: i32 = ...;` | 无效文件描述符 |
| `EBUSY` | `pub const EBUSY: i32 = ...;` | 设备/资源忙 |
| `EINTR` | `pub const EINTR: i32 = ...;` | 系统调用被信号中断 |
| `EACCES` | `pub const EACCES: i32 = ...;` | 权限不足 |
| `EPERM` | `pub const EPERM: i32 = ...;` | 操作不允许 |
| `ENOENT` | `pub const ENOENT: i32 = ...;` | 文件/目录不存在 |
| `ENODEV` | `pub const ENODEV: i32 = ...;` | 设备不匹配 |
| `ENXIO` | `pub const ENXIO: i32 = ...;` | 无此设备或地址 |
| `ERANGE` | `pub const ERANGE: i32 = ...;` | 结果超出范围 |
| `EAGAIN` | `pub const EAGAIN: i32 = ...;` | 资源暂时不可用 |

这些常量定义在 `rusl::errno` 模块中，`rusl::unistd` 通过 `use rusl::errno::*;` 导入。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  上述所有 syscall 接口、跨模块函数、结构体和类型均由 rusl 其他模块提供
Predefined Macros/Crates:
  core::ffi — Rust 核心库的 FFI 类型
  rusl::syscall — rusl 内部系统调用封装模块
  rusl::errno — rusl 内部错误码管理模块
  rusl::sys — rusl 内部系统类型定义模块

[GUARANTEE]
Exported Interface:
  本文件仅记录依赖，不对外导出任何符号
