# faccessat.c 规约

> musl libc POSIX 相对路径文件访问权限检查系统调用封装。`faccessat` 在 `<unistd.h>` 中声明。实现包含复杂的回退逻辑以支持 `AT_EACCESS` 标志。

---

## 依赖图

```
faccessat (Public)
  ├── flag!=0 时: __syscall(SYS_faccessat2, fd, filename, amode, flag) — 尝试新 syscall
  │     └── 成功: __syscall_ret(ret)
  │     └── ENOSYS: 检查 flag 有效性
  ├── flag & ~AT_EACCESS: 返回 -EINVAL
  ├── flag==0 或 uid==euid 且 gid==egid: syscall(SYS_faccessat, fd, filename, amode)
  └── flag & AT_EACCESS 且 uid!=euid:
        ├── pipe2(p, O_CLOEXEC) — 创建通信管道
        ├── __clone(checker, stack, 0, &ctx) — 在子进程中检查
        │     └── checker: 设置 uid/gid → SYS_faccessat → write 结果到管道
        ├── __syscall(SYS_read, p[0], &ret, sizeof ret) — 读取子进程结果
        ├── __sys_wait4(pid, &status, __WCLONE, 0) — 等待子进程
        └── __syscall_ret(ret)
        （全过程在 __block_all_sigs / __restore_sigs 保护下执行）
```

---

## 函数规约

### faccessat

```c
int faccessat(int fd, const char *filename, int amode, int flag);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

相对于目录文件描述符 `fd` 检查文件的访问权限。支持 `AT_EACCESS` 标志（使用有效而非真实 UID/GID 检查）和 `AT_SYMLINK_NOFOLLOW` 标志（不跟随符号链接）。

当需要 `AT_EACCESS` 且调用者的真实和有效 UID/GID 不同时，musl 通过 `__clone` 创建子进程并在子进程中临时切换身份来执行检查，因为内核空间不支持 `setreuid` 后的真实 UID 访问检查。

#### 前置条件

- `fd`: 有效目录文件描述符，或 `AT_FDCWD`（相对于当前工作目录）
- `filename`: 以 NULL 结尾的有效相对/绝对路径
- `amode`: `F_OK`、`R_OK|W_OK|X_OK` 的组合
- `flag`: 0、`AT_EACCESS`（使用有效 ID 检查）或 `AT_SYMLINK_NOFOLLOW` 的组合

#### 后置条件

- **Case 1 允许访问**: 返回 0
- **Case 2 错误**: 返回 -1，设置 errno

#### 系统算法

```
faccessat(fd, filename, amode, flag):
  if flag:
    ret = __syscall(SYS_faccessat2, fd, filename, amode, flag)  // Linux 5.8+ 原生支持
    if ret != -ENOSYS: return __syscall_ret(ret)

  if flag & ~AT_EACCESS: return __syscall_ret(-EINVAL)          // 只支持 AT_EACCESS 和 AT_SYMLINK_NOFOLLOW

  if !flag || (getuid()==geteuid() && getgid()==getegid()):     // 无需特殊处理
    return syscall(SYS_faccessat, fd, filename, amode)

  // GETUID!=GETEUID 情况：使用子进程进行真实 ID 访问检查
  pipe2(p, O_CLOEXEC)
  __block_all_sigs(&set)
  pid = __clone(checker, stack, 0, &ctx)                         // 子进程执行 checker
  读管道获得子进程结果
  __sys_wait4(pid, &status, __WCLONE, 0)                         // 回收子进程
  __restore_sigs(&set)
  return __syscall_ret(ret)
```

#### 子进程 checker 算法

```
checker(ctx):
  // 设置有效 ID 等于真实 ID（让内核用真实 ID 检查）
  __syscall(SYS_setregid, SYS_getegid, -1)
  __syscall(SYS_setreuid, SYS_geteuid, -1)
  ret = __syscall(SYS_faccessat, fd, filename, amode, 0)         // flag=0 执行实际检查
  __syscall(SYS_write, pipe_fd, &ret, sizeof ret)                // 写回结果
  return 0
```

#### 不变量

- `AT_EACCESS` 回退路径：信号全阻塞期间，父子进程通过管道通信确保结果的正确传递

#### 依赖

- `SYS_faccessat2` — Linux 5.8+ 系统调用（原生支持 AT_EACCESS）
- `SYS_faccessat` — Linux 内核系统调用
- `__clone` — 创建子进程
- `__sys_wait4` — 等待子进程
- `__block_all_sigs` / `__restore_sigs` — 信号阻塞管理
- `SYS_setregid` / `SYS_setreuid` — 临时切换用户/组 ID
- `pipe2` / `SYS_read` / `SYS_write` / `SYS_close` — 父子进程间通信
- `getuid` / `geteuid` / `getgid` / `getegid` — 用户/组 ID 查询
