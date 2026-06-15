# unistd 模块 — 外部依赖导入

本文件记录 `src/unistd/` 下所有函数使用到的来自外部模块的 C 接口。

---

## 系统调用接口（来自内核）

| 接口 | 说明 |
|------|------|
| `SYS_read` / `syscall_cp(SYS_read, fd, buf, count)` | 内核文件读取系统调用 |
| `SYS_write` / `syscall_cp(SYS_write, fd, buf, count)` | 内核文件写入系统调用 |
| `SYS_close` / `__syscall_cp(SYS_close, fd)` | 内核关闭文件描述符系统调用 |
| `SYS_dup` / `syscall(SYS_dup, fd)` | 内核复制文件描述符系统调用 |
| `SYS_dup2` / `__syscall(SYS_dup2, old, new)` | 内核复制文件描述符系统调用（指定编号） |
| `SYS_dup3` / `__syscall(SYS_dup3, old, new, flags)` | 内核带标志复制文件描述符系统调用 |
| `SYS_lseek` / `syscall(SYS_lseek, fd, offset, whence)` | 内核文件定位系统调用 |
| `SYS__llseek` / `syscall(SYS__llseek, ...)` | 内核 64 位文件定位系统调用（32 位平台） |
| `SYS_pipe` / `syscall(SYS_pipe, fd)` | 内核管道创建系统调用 |
| `SYS_pipe2` / `__syscall(SYS_pipe2, fd, flag)` | 内核带标志管道创建系统调用 |
| `SYS_pread` / `syscall_cp(SYS_pread, fd, buf, size, ofs)` | 内核指定偏移读取系统调用 |
| `SYS_pwrite` / `syscall_cp(SYS_pwrite, fd, buf, size, ofs)` | 内核指定偏移写入系统调用 |
| `SYS_pwritev2` / `__syscall_cp(SYS_pwritev2, ...)` | 内核扩展偏移聚集写入系统调用 |
| `SYS_readv` / `syscall_cp(SYS_readv, fd, iov, count)` | 内核分散读取系统调用 |
| `SYS_writev` / `syscall_cp(SYS_writev, fd, iov, count)` | 内核聚集写入系统调用 |
| `SYS_preadv` / `syscall_cp(SYS_preadv, fd, iov, count, ...)` | 内核指定偏移分散读取系统调用 |
| `SYS_pwritev` / `syscall_cp(SYS_pwritev, fd, iov, count, ...)` | 内核指定偏移聚集写入系统调用 |
| `SYS_fsync` / `syscall_cp(SYS_fsync, fd)` | 内核同步文件元数据系统调用 |
| `SYS_fdatasync` / `syscall_cp(SYS_fdatasync, fd)` | 内核同步文件数据系统调用 |
| `SYS_ftruncate` / `syscall(SYS_ftruncate, fd, length)` | 内核文件截断系统调用 |
| `SYS_truncate` / `syscall(SYS_truncate, path, length)` | 内核路径文件截断系统调用 |
| `SYS_access` / `syscall(SYS_access, filename, amode)` | 内核文件访问检查系统调用 |
| `SYS_faccessat` / `syscall(SYS_faccessat, fd, filename, amode, flag)` | 内核相对路径访问检查系统调用 |
| `SYS_faccessat2` / `__syscall(SYS_faccessat2, fd, filename, amode, flag)` | 内核扩展访问检查系统调用（Linux 5.8+） |
| `SYS_chdir` / `syscall(SYS_chdir, path)` | 内核改变工作目录系统调用 |
| `SYS_fchdir` / `__syscall(SYS_fchdir, fd)` | 内核文件描述符改变工作目录系统调用 |
| `SYS_getcwd` / `syscall(SYS_getcwd, buf, size)` | 内核获取工作目录系统调用 |
| `SYS_chown` / `syscall(SYS_chown, path, uid, gid)` | 内核改变所有者系统调用 |
| `SYS_fchown` / `__syscall(SYS_fchown, fd, uid, gid)` | 内核描述符改变所有者系统调用 |
| `SYS_lchown` / `syscall(SYS_lchown, path, uid, gid)` | 内核符号链接改变所有者系统调用 |
| `SYS_fchownat` / `syscall(SYS_fchownat, fd, path, uid, gid, flag)` | 内核相对路径改变所有者系统调用 |
| `SYS_link` / `syscall(SYS_link, existing, new)` | 内核创建硬链接系统调用 |
| `SYS_linkat` / `syscall(SYS_linkat, fd1, existing, fd2, new, flag)` | 内核相对路径创建硬链接系统调用 |
| `SYS_symlink` / `syscall(SYS_symlink, existing, new)` | 内核创建符号链接系统调用 |
| `SYS_symlinkat` / `syscall(SYS_symlinkat, existing, fd, new)` | 内核相对路径创建符号链接系统调用 |
| `SYS_readlink` / `__syscall(SYS_readlink, path, buf, bufsize)` | 内核读取符号链接系统调用 |
| `SYS_readlinkat` / `__syscall(SYS_readlinkat, fd, path, buf, bufsize)` | 内核相对路径读取符号链接系统调用 |
| `SYS_unlink` / `syscall(SYS_unlink, path)` | 内核删除文件系统调用 |
| `SYS_unlinkat` / `syscall(SYS_unlinkat, fd, path, flag)` | 内核相对路径删除文件系统调用 |
| `SYS_rmdir` / `syscall(SYS_rmdir, path)` | 内核删除目录系统调用 |
| `SYS_renameat` / `syscall(SYS_renameat, oldfd, old, newfd, new)` | 内核相对路径重命名系统调用 |
| `SYS_renameat2` / `syscall(SYS_renameat2, oldfd, old, newfd, new, flags)` | 内核带标志重命名系统调用 |
| `SYS_getpid` / `__syscall(SYS_getpid)` | 内核获取进程 ID 系统调用 |
| `SYS_getppid` / `__syscall(SYS_getppid)` | 内核获取父进程 ID 系统调用 |
| `SYS_getpgid` / `__syscall(SYS_getpgid, pid)` 或 `syscall(SYS_getpgid, pid)` | 内核获取进程组 ID 系统调用 |
| `SYS_setpgid` / `syscall(SYS_setpgid, pid, pgid)` | 内核设置进程组 ID 系统调用 |
| `SYS_setsid` / `syscall(SYS_setsid)` | 内核创建新会话系统调用 |
| `SYS_getsid` / `syscall(SYS_getsid, pid)` | 内核获取会话 ID 系统调用 |
| `SYS_getuid` / `__syscall(SYS_getuid)` | 内核获取真实用户 ID 系统调用 |
| `SYS_geteuid` / `__syscall(SYS_geteuid)` | 内核获取有效用户 ID 系统调用 |
| `SYS_getgid` / `__syscall(SYS_getgid)` | 内核获取真实组 ID 系统调用 |
| `SYS_getegid` / `__syscall(SYS_getegid)` | 内核获取有效组 ID 系统调用 |
| `SYS_getgroups` / `syscall(SYS_getgroups, count, list)` | 内核获取附加组列表系统调用 |
| `SYS_setuid` / `__syscall(SYS_setuid, uid, ...)` (通过 `__setxid`) | 内核设置用户 ID 系统调用 |
| `SYS_setgid` / `__syscall(SYS_setgid, gid, ...)` (通过 `__setxid`) | 内核设置组 ID 系统调用 |
| `SYS_setreuid` / `__syscall(SYS_setreuid, ruid, euid, ...)` (通过 `__setxid`) | 内核设置真实/有效用户 ID |
| `SYS_setregid` / `__syscall(SYS_setregid, rgid, egid, ...)` (通过 `__setxid`) | 内核设置真实/有效组 ID |
| `SYS_setresuid` / `__syscall(SYS_setresuid, ruid, euid, suid)` (通过 `__setxid`) | 内核设置真实/有效/保存用户 ID |
| `SYS_setresgid` / `__syscall(SYS_setresgid, rgid, egid, sgid)` (通过 `__setxid`) | 内核设置真实/有效/保存组 ID |
| `SYS_ioctl` / `syscall(SYS_ioctl, fd, req, ...)` | 内核设备控制操作 |
| `SYS_fcntl` / `__syscall(SYS_fcntl, fd, cmd, ...)` | 内核文件描述符控制 |
| `SYS_sync` / `__syscall(SYS_sync)` | 内核同步所有文件系统缓冲区 |
| `SYS_acct` / `syscall(SYS_acct, filename)` | 内核进程记账系统调用 |
| `SYS_exit` / `__syscall(SYS_exit, code)` | 内核进程退出系统调用 |
| `SYS_kill` / `__syscall(SYS_kill, pid, sig)` | 内核发送信号系统调用 |
| `SYS_clone` | 内核创建子进程系统调用 |
| `SYS_wait4` / `__sys_wait4(pid, status, options, ...)` | 内核等待子进程系统调用 |

---

## 结构体与类型

| 类型 | 来源 | 说明 |
|------|------|------|
| `struct iovec` | `<sys/uio.h>` | 散布/聚集 I/O 向量 |
| `struct itimerval` | `<sys/time.h>` | 间隔定时器值结构 |
| `struct utsname` | `<sys/utsname.h>` | 系统名称信息结构 |
| `struct timespec` | `<time.h>` | POSIX 时间规范结构 |
| `struct winsize` | `<sys/ioctl.h>` | 终端窗口大小结构 |
| `struct stat` | `<sys/stat.h>` | 文件状态结构 |
| `sigset_t` | `<signal.h>` | 信号集类型 |
| `size_t` / `ssize_t` | `<stddef.h>` / `<sys/types.h>` | 标准大小类型 |
| `pid_t` / `uid_t` / `gid_t` / `off_t` | `<sys/types.h>` | POSIX 标准类型 |

---

## libc 函数

| 接口 | 来源 | 说明 |
|------|------|------|
| `setitimer(which, new, old)` | `<sys/time.h>` | 设置间隔定时器 |
| `getpriority(which, who)` | `<sys/resource.h>` | 获取进程调度优先级 |
| `setpriority(which, who, prio)` | `<sys/resource.h>` | 设置进程调度优先级 |
| `uname(buf)` | `<sys/utsname.h>` | 获取系统名称信息 |
| `getenv(name)` | `<stdlib.h>` | 获取环境变量值 |
| `strcpy(dst, src)` | `<string.h>` | 字符串拷贝 |
| `strlen(s)` | `<string.h>` | 计算字符串长度 |
| `strdup(s)` | `<stdlib.h>` | 字符串复制到堆内存 |
| `nanosleep(req, rem)` | `<time.h>` | 高精度睡眠 |
| `ioctl(fd, req, ...)` | `<sys/ioctl.h>` | 设备控制操作 |
| `readlink(path, buf, size)` | `<unistd.h>` | 读取符号链接目标 |
| `stat(path, buf)` | `<sys/stat.h>` | 获取文件状态 |
| `fstat(fd, buf)` | `<sys/stat.h>` | 通过文件描述符获取文件状态 |
| `fcntl(fd, cmd, ...)` | `<fcntl.h>` | 文件描述符控制 |
| `_Exit(status)` | `<stdlib.h>` | 立即终止进程（ISO C） |
| `pipe(fd)` | `<unistd.h>` | 创建管道（pipe2 内部回退） |
| `close(fd)` | `<unistd.h>` | 关闭文件描述符（posix_close 回退） |
| `getuid()` / `geteuid()` | `<unistd.h>` | 获取用户 ID（faccessat 权限检查） |
| `getgid()` / `getegid()` | `<unistd.h>` | 获取组 ID（faccessat 权限检查） |
| `pipe2(fd, flags)` | `<unistd.h>` | 带标志创建管道（faccessat 内部使用） |
| `setpgid(pid, pgid)` | `<unistd.h>` | 设置进程组（setpgrp 回退） |
| `ttyname_r(fd, buf, size)` | `<unistd.h>` | 获取终端名（ttyname 回退） |
| `isatty(fd)` | `<unistd.h>` | 测试终端（ttyname_r 依赖） |
| `getlogin()` | `<unistd.h>` | 获取登录名（getlogin_r 回退） |

---

## 跨模块内部依赖

| 接口 | 定义位置 | 说明 |
|------|----------|------|
| `__syscall_ret(r)` | `src/internal/syscall_ret.c` | 内核返回值到 libc 错误码转换 |
| `__syscall_cp(...)` | `src/internal/syscall_cp.c` | 可被信号取消的系统调用封装 |
| `syscall_cp(...)` | `src/internal/syscall.h` 宏 | `__syscall_ret(__syscall_cp(...))` |
| `syscall(...)` | `src/internal/syscall.h` 宏 | `__syscall_ret(__syscall(...))` |
| `__syscall(...)` | `src/internal/syscall.h` 宏 | 原始系统调用 |
| `__SYSCALL_LL_PRW(ofs)` | `src/internal/syscall.h` 宏 | 32 位/64 位 pread/pwrite 偏移展开 |
| `__SYSCALL_LL_O(length)` | `src/internal/syscall.h` 宏 | 32 位/64 位 truncate/ftruncate 长度展开 |
| `__procfdname(buf, fd)` | `src/internal/procfdname.c` | 构造 `/proc/self/fd/<fd>` 路径字符串 |
| `__block_all_sigs(set)` | `src/signal/block.c` | 阻塞所有信号并保存原信号掩码 |
| `__restore_sigs(set)` | `src/signal/block.c` | 恢复之前保存的信号掩码 |
| `__synccall(func, ctx)` | `src/internal/__synccall.c` | 在所有线程上同步执行函数 |
| `__setxid(nr, id, eid, sid)` | `src/unistd/setxid.c` | 跨线程设置用户/组 ID（内部函数） |
| `sys_pause_cp()` | `src/internal/` 内部封装 | 可被信号取消的 pause 调用 |
| `__aio_close(fd)` | `src/aio/aio.c`（弱别名默认 dummy） | AIO 关闭回调 |
| `__sys_wait4(pid, status, options, rusage)` | `src/internal/` 内部封装 | 等待子进程状态变化 |
| `__clone(func, stack, flags, arg)` | `src/internal/` 内部封装 | 创建子进程 |

---

## 宏与常量

| 宏/常量 | 来源 | 说明 |
|----------|------|------|
| `AT_FDCWD` | `<fcntl.h>` | 相对于当前工作目录的特殊 fd 值 |
| `AT_EACCESS` | `<fcntl.h>` | faccessat: 使用有效 UID/GID 检查 |
| `AT_SYMLINK_NOFOLLOW` | `<fcntl.h>` | fchownat/unlinkat: 不跟随符号链接 |
| `AT_REMOVEDIR` | `<fcntl.h>` | unlinkat: 删除目录而非文件 |
| `O_CLOEXEC` | `<fcntl.h>` | 新 fd 设置 close-on-exec 标志 |
| `O_NONBLOCK` | `<fcntl.h>` | 非阻塞 I/O 标志 |
| `O_APPEND` | `<fcntl.h>` | 追加模式标志 |
| `FD_CLOEXEC` | `<fcntl.h>` | 文件描述符 close-on-exec 标志 |
| `F_GETFD` / `F_SETFD` / `F_GETFL` / `F_SETFL` | `<fcntl.h>` | fcntl 操作命令 |
| `TIOCGWINSZ` | `<sys/ioctl.h>` | 获取终端窗口大小的 ioctl 请求 |
| `TIOCGPGRP` | `<sys/ioctl.h>` (通过 `<termios.h>`) | 获取前台进程组的 ioctl 请求 |
| `TIOCSPGRP` | `<sys/ioctl.h>` (通过 `<termios.h>`) | 设置前台进程组的 ioctl 请求 |
| `TTY_NAME_MAX` | `<limits.h>` | 终端名称最大长度 |
| `PATH_MAX` | `<limits.h>` | 路径最大长度 |
| `ITIMER_REAL` | `<sys/time.h>` | 真实时间定时器类型 |
| `PRIO_PROCESS` | `<sys/resource.h>` | 进程优先级的 who 参数类型 |
| `NZERO` | `<sys/resource.h>` | 默认优先级（通常 20） |
| `RWF_NOAPPEND` | `<fcntl.h>` | pwritev2: 忽略 O_APPEND 标志 |
| `ENOSYS` | `<errno.h>` | 系统调用未实现 |
| `EINVAL` | `<errno.h>` | 无效参数 |
| `EBADF` | `<errno.h>` | 无效文件描述符 |
| `EBUSY` | `<errno.h>` | 设备/资源忙 |
| `EINTR` | `<errno.h>` | 系统调用被信号中断 |
| `EOPNOTSUPP` | `<errno.h>` | 操作不支持 |
| `EACCES` / `EPERM` | `<errno.h>` | 权限错误（nice 映射 EACCES→EPERM） |
| `ENOENT` | `<errno.h>` | 文件/目录不存在 |
| `ENODEV` | `<errno.h>` | 设备不匹配（ttyname_r 设备检查） |
| `ENXIO` | `<errno.h>` | 无此设备或地址（getlogin_r） |
| `ERANGE` | `<errno.h>` | 结果超出范围（getlogin_r/ttyname_r） |
| `EAGAIN` | `<errno.h>` | 资源暂时不可用（__setxid 并发失败） |
| `SIGKILL` | `<signal.h>` | 不可捕获/忽略的终止信号 |
| `__WCLONE` | `<sys/wait.h>` | 等待克隆的子进程标志 |
