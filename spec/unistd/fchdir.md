# fchdir.c 规约

> musl libc POSIX 通过文件描述符改变工作目录系统调用封装。`fchdir` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
fchdir (Public)
  ├── __syscall(SYS_fchdir, fd) — 尝试直接 fchdir
  │     └── 非 -EBADF: __syscall_ret(ret)
  │     └── -EBADF: 检查 fd 是否有效
  │           ├── __syscall(SYS_fcntl, fd, F_GETFD) < 0: fd 真的无效 → 返回 EBADF
  │           └── fd 有效 → 通过 /proc/self/fd/<fd> 回退
  │                 ├── __procfdname(buf, fd) — 构造 /proc 路径
  │                 └── syscall(SYS_chdir, buf) — 通过路径名 chdir
  └── [回退] 通过 /proc/self/fd/<fd> 符号链接执行 chdir
```

---

## 函数规约

### fchdir

```c
int fchdir(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将调用进程的当前工作目录改为文件描述符 `fd` 所引用的目录。musl 实现了特殊的回退逻辑：当内核直接拒绝 `fchdir`（`EBADF`）但 `fd` 实际有效时（某些文件系统/proc 上的目录可能不支持 fchdir），通过 `/proc/self/fd/<fd>` 路径名执行 `chdir`。

#### 前置条件

- `fd`: 有效的已打开目录文件描述符

#### 后置条件

- **Case 1 成功**: 当前工作目录变更，返回 0
- **Case 2 fd 无效**: 返回 -1，errno = `EBADF`
- **Case 3 其他错误**: 返回 -1，errno 设置（如 `EACCES`、`ENOENT` 等）

#### 系统算法

```
fchdir(fd):
  ret = __syscall(SYS_fchdir, fd)                    // 尝试内核 fchdir
  if ret != -EBADF: return __syscall_ret(ret)         // 成功或明确错误

  if __syscall(SYS_fcntl, fd, F_GETFD) < 0:          // fd 确实无效
    return __syscall_ret(ret)                          // 返回 EBADF

  // fd 有效但 fchdir 返回 EBADF → 通过 /proc 回退
  __procfdname(buf, fd)                               // 构造 "/proc/self/fd/<fd>"
  return syscall(SYS_chdir, buf)                      // 通过路径执行 chdir
```

#### 依赖

- `SYS_fchdir` — Linux 内核系统调用
- `__procfdname` — 构造 /proc/self/fd/ 路径
- `SYS_fcntl` + `F_GETFD` — 检查 fd 有效性
- `SYS_chdir` — 回退路径
