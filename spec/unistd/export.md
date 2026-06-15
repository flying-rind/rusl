# unistd 模块 — 对外导出 API 汇总

本文件记录 `src/unistd/` 下所有在 `<unistd.h>` 中声明的、用户程序可直接调用的公开接口。内部实现符号（`__` 前缀、`hidden` 可见性、弱别名等）不在此列，详见各具体 spec 文件。

---

## 宏常量

| 符号 | 类型 | 值 | 说明 |
|------|------|-----|------|
| `STDIN_FILENO` | macro | `0` | 标准输入文件描述符 |
| `STDOUT_FILENO` | macro | `1` | 标准输出文件描述符 |
| `STDERR_FILENO` | macro | `2` | 标准错误输出文件描述符 |
| `SEEK_SET` | macro | `0` | lseek: 从文件起始偏移 |
| `SEEK_CUR` | macro | `1` | lseek: 从当前位置偏移 |
| `SEEK_END` | macro | `2` | lseek: 从文件末尾偏移 |
| `SEEK_DATA` | macro | `3` | lseek: 定位到下一个数据区域 |
| `SEEK_HOLE` | macro | `4` | lseek: 定位到下一个空洞区域 |
| `F_OK` | macro | `0` | access: 测试文件存在性 |
| `R_OK` | macro | `4` | access: 测试读权限 |
| `W_OK` | macro | `2` | access: 测试写权限 |
| `X_OK` | macro | `1` | access: 测试执行权限 |
| `_POSIX_VERSION` | macro | `200809L` | POSIX.1-2008 版本标识 |
| `POSIX_CLOSE_RESTART` | macro | `0` | posix_close 标志 |

---

## 类型定义

| 符号 | 说明 |
|------|------|
| `size_t` | 无符号整数，对象大小类型 |
| `ssize_t` | 有符号整数，字节计数/错误返回类型 |
| `uid_t` | 用户 ID 类型 |
| `gid_t` | 组 ID 类型 |
| `off_t` | 文件偏移类型（64 位） |
| `pid_t` | 进程 ID 类型 |
| `intptr_t` | 指针大小的有符号整数 |
| `useconds_t` | 微秒无符号整数 |

---

## 1. 基本 I/O

| 符号 | 签名 | 说明 |
|------|------|------|
| `read` | `ssize_t read(int fd, void *buf, size_t count)` | 从文件描述符读取数据 |
| `write` | `ssize_t write(int fd, const void *buf, size_t count)` | 向文件描述符写入数据 |
| `close` | `int close(int fd)` | 关闭文件描述符 |
| `dup` | `int dup(int fd)` | 复制文件描述符 |
| `dup2` | `int dup2(int old, int new)` | 复制文件描述符到指定编号 |
| `dup3` | `int dup3(int old, int new, int flags)` | 带标志复制文件描述符（GNU） |
| `lseek` | `off_t lseek(int fd, off_t offset, int whence)` | 移动文件读写位置 |
| `pipe` | `int pipe(int fd[2])` | 创建管道 |
| `pipe2` | `int pipe2(int fd[2], int flags)` | 带标志创建管道 |
| `pread` | `ssize_t pread(int fd, void *buf, size_t size, off_t ofs)` | 从指定偏移读取（原子操作） |
| `pwrite` | `ssize_t pwrite(int fd, const void *buf, size_t size, off_t ofs)` | 向指定偏移写入（原子操作） |
| `readv` | `ssize_t readv(int fd, const struct iovec *iov, int count)` | 分散读取 |
| `writev` | `ssize_t writev(int fd, const struct iovec *iov, int count)` | 聚集写入 |
| `preadv` | `ssize_t preadv(int fd, const struct iovec *iov, int count, off_t ofs)` | 从指定偏移分散读取（BSD） |
| `pwritev` | `ssize_t pwritev(int fd, const struct iovec *iov, int count, off_t ofs)` | 向指定偏移聚集写入（GNU） |
| `fsync` | `int fsync(int fd)` | 同步文件数据和元数据到磁盘 |
| `fdatasync` | `int fdatasync(int fd)` | 同步文件数据到磁盘 |
| `ftruncate` | `int ftruncate(int fd, off_t length)` | 截断文件到指定长度 |
| `posix_close` | `int posix_close(int fd, int flags)` | POSIX 关闭文件描述符 |

## 2. 文件系统操作

| 符号 | 签名 | 说明 |
|------|------|------|
| `access` | `int access(const char *filename, int amode)` | 检查文件访问权限 |
| `faccessat` | `int faccessat(int fd, const char *filename, int amode, int flag)` | 相对于目录 fd 检查文件访问权限 |
| `chdir` | `int chdir(const char *path)` | 改变当前工作目录 |
| `fchdir` | `int fchdir(int fd)` | 通过文件描述符改变当前工作目录 |
| `getcwd` | `char *getcwd(char *buf, size_t size)` | 获取当前工作目录路径 |
| `chown` | `int chown(const char *path, uid_t uid, gid_t gid)` | 改变文件所有者 |
| `fchown` | `int fchown(int fd, uid_t uid, gid_t gid)` | 通过文件描述符改变文件所有者 |
| `lchown` | `int lchown(const char *path, uid_t uid, gid_t gid)` | 改变符号链接自身所有者 |
| `fchownat` | `int fchownat(int fd, const char *path, uid_t uid, gid_t gid, int flag)` | 相对于目录 fd 改变文件所有者 |
| `link` | `int link(const char *existing, const char *new)` | 创建硬链接 |
| `linkat` | `int linkat(int fd1, const char *existing, int fd2, const char *new, int flag)` | 相对于目录 fd 创建硬链接 |
| `symlink` | `int symlink(const char *existing, const char *new)` | 创建符号链接 |
| `symlinkat` | `int symlinkat(const char *existing, int fd, const char *new)` | 相对于目录 fd 创建符号链接 |
| `readlink` | `ssize_t readlink(const char *restrict path, char *restrict buf, size_t bufsize)` | 读取符号链接目标路径 |
| `readlinkat` | `ssize_t readlinkat(int fd, const char *restrict path, char *restrict buf, size_t bufsize)` | 相对于目录 fd 读取符号链接目标 |
| `unlink` | `int unlink(const char *path)` | 删除文件名（文件） |
| `unlinkat` | `int unlinkat(int fd, const char *path, int flag)` | 相对于目录 fd 删除文件名 |
| `rmdir` | `int rmdir(const char *path)` | 删除空目录 |
| `renameat` | `int renameat(int oldfd, const char *old, int newfd, const char *new)` | 相对于目录 fd 重命名文件 |
| `truncate` | `int truncate(const char *path, off_t length)` | 通过路径截断文件 |
| `sync` | `void sync(void)` | 同步所有文件系统缓冲区（BSD/XOPEN） |
| `acct` | `int acct(const char *filename)` | 启用/禁用进程记账（GNU） |

## 3. 进程控制

| 符号 | 签名 | 说明 |
|------|------|------|
| `_exit` | `_Noreturn void _exit(int status)` | 立即终止进程（无清理） |
| `alarm` | `unsigned alarm(unsigned seconds)` | 设置 SIGALRM 定时器 |
| `sleep` | `unsigned sleep(unsigned seconds)` | 睡眠指定秒数 |
| `pause` | `int pause(void)` | 等待信号 |
| `getpid` | `pid_t getpid(void)` | 获取当前进程 ID |
| `getppid` | `pid_t getppid(void)` | 获取父进程 ID |
| `getpgrp` | `pid_t getpgrp(void)` | 获取进程组 ID |
| `getpgid` | `pid_t getpgid(pid_t pid)` | 获取指定进程的进程组 ID |
| `setpgid` | `int setpgid(pid_t pid, pid_t pgid)` | 设置进程组 ID |
| `setsid` | `pid_t setsid(void)` | 创建新会话 |
| `getsid` | `pid_t getsid(pid_t pid)` | 获取指定进程的会话 ID |
| `setpgrp` | `pid_t setpgrp(void)` | 设置进程组（BSD/XOPEN，等价 setpgid(0,0)） |
| `nice` | `int nice(int inc)` | 调整进程调度优先级（BSD/XOPEN） |

## 4. 用户标识和组

| 符号 | 签名 | 说明 |
|------|------|------|
| `getuid` | `uid_t getuid(void)` | 获取真实用户 ID |
| `geteuid` | `uid_t geteuid(void)` | 获取有效用户 ID |
| `getgid` | `gid_t getgid(void)` | 获取真实组 ID |
| `getegid` | `gid_t getegid(void)` | 获取有效组 ID |
| `getgroups` | `int getgroups(int count, gid_t list[])` | 获取附加组列表 |
| `setuid` | `int setuid(uid_t uid)` | 设置用户 ID |
| `seteuid` | `int seteuid(uid_t euid)` | 设置有效用户 ID |
| `setgid` | `int setgid(gid_t gid)` | 设置组 ID |
| `setegid` | `int setegid(gid_t egid)` | 设置有效组 ID |
| `setreuid` | `int setreuid(uid_t ruid, uid_t euid)` | 设置真实和有效用户 ID（BSD/XOPEN） |
| `setregid` | `int setregid(gid_t rgid, gid_t egid)` | 设置真实和有效组 ID（BSD/XOPEN） |

## 5. 用户标识和组 — GNU 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `setresuid` | `int setresuid(uid_t ruid, uid_t euid, uid_t suid)` | 设置真实/有效/保存用户 ID（GNU） |
| `setresgid` | `int setresgid(gid_t rgid, gid_t egid, gid_t sgid)` | 设置真实/有效/保存组 ID（GNU） |

## 6. 终端操作

| 符号 | 签名 | 说明 |
|------|------|------|
| `ttyname` | `char *ttyname(int fd)` | 获取终端设备名称 |
| `ttyname_r` | `int ttyname_r(int fd, char *name, size_t size)` | 获取终端设备名称（线程安全版） |
| `isatty` | `int isatty(int fd)` | 测试文件描述符是否关联终端 |
| `tcgetpgrp` | `pid_t tcgetpgrp(int fd)` | 获取终端前台进程组 |
| `tcsetpgrp` | `int tcsetpgrp(int fd, pid_t pgrp)` | 设置终端前台进程组 |
| `ctermid` | `char *ctermid(char *s)` | 获取控制终端路径名 |

## 7. 其他系统信息

| 符号 | 签名 | 说明 |
|------|------|------|
| `gethostname` | `int gethostname(char *name, size_t len)` | 获取主机名 |
| `getlogin` | `char *getlogin(void)` | 获取登录用户名 |
| `getlogin_r` | `int getlogin_r(char *name, size_t size)` | 获取登录用户名（线程安全版） |

## 8. BSD/GNU 扩展 — 时间

| 符号 | 签名 | 说明 |
|------|------|------|
| `usleep` | `int usleep(unsigned useconds)` | 睡眠指定微秒数 |
| `ualarm` | `unsigned ualarm(unsigned value, unsigned interval)` | 设置微秒级间隔定时器 |

## 9. _LARGEFILE64_SOURCE 兼容宏

以下宏在定义 `_LARGEFILE64_SOURCE` 时生效，将 `xxx64` 符号映射到对应的标准函数/类型：

| 宏 | 映射目标 | 说明 |
|-----|---------|------|
| `lseek64` | `lseek` | 64 位文件定位（透明别名） |
| `pread64` | `pread` | 64 位偏移读取（透明别名） |
| `pwrite64` | `pwrite` | 64 位偏移写入（透明别名） |
| `truncate64` | `truncate` | 64 位文件截断（透明别名） |
| `ftruncate64` | `ftruncate` | 64 位文件描述符截断（透明别名） |
| `off64_t` | `off_t` | 64 位文件偏移类型（透明别名） |

---

## 排除说明

以下类别的符号**不出现在**本文件中，它们属于内部实现细节：

- 所有 `__` 前缀的内部函数（如 `__lseek`、`__dup3`、`__setxid` 等）
- `weak_alias` 定义的弱别名（如 `lseek` 是 `__lseek` 的弱别名）
- `hidden` 可见性的内部符号
- 内部静态变量及结构体实例（如 ttyname 的静态缓冲区）
- 架构特定实现（如 `mips/pipe.s`、`x32/lseek.c` 等）
- 全局符号：`optarg`、`optind`、`opterr`、`optopt`、`optreset`、`environ`（由 getopt 等模块管理）
