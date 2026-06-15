# pipe2.c 规约

> musl libc Linux 特有带标志的管道创建系统调用封装。`pipe2` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
pipe2 (Public)
  ├── flag==0: pipe(fd) — 回退到 pipe()
  └── flag!=0: __syscall(SYS_pipe2, fd, flag)
        ├── 成功或非 ENOSYS: __syscall_ret(ret)
        └── ENOSYS 回退:
              ├── flag 含非法位: __syscall_ret(-EINVAL)
              ├── pipe(fd) — 先创建管道
              ├── flags & O_CLOEXEC: __syscall(SYS_fcntl, fd[0/1], F_SETFD, FD_CLOEXEC)
              └── flags & O_NONBLOCK: __syscall(SYS_fcntl, fd[0/1], F_SETFL, O_NONBLOCK)
```

---

## 函数规约

### pipe2

```c
int pipe2(int fd[2], int flag);
```

[Visibility]: User — `<unistd.h>` Linux 特有函数（`_GNU_SOURCE`），用户程序可直接调用

#### Intent

创建一对管道文件描述符，并原子性地设置指定标志。比先 pipe() 再 fcntl() 更安全，避免了竞态条件。支持的 flag 包括 `O_CLOEXEC`（close-on-exec）和 `O_NONBLOCK`（非阻塞 I/O）。

#### 前置条件

- `fd`: 指向至少 2 个 `int` 数组的非空指针
- `flag`: 0 或 `O_CLOEXEC | O_NONBLOCK` 的组合

#### 后置条件

- **Case 1 成功（flag == 0）**
  - 行为等同于 `pipe(fd)`
  - 返回 0

- **Case 2 成功（flag != 0）**
  - `fd[0]` 为管道读端，`fd[1]` 为管道写端
  - 根据 flag 设置 close-on-exec 和/或非阻塞标志
  - 返回 0

- **Case 3 flag 包含不支持的位（且 SYS_pipe2 不可用）**
  - 返回 -1
  - `errno` 设置为 `EINVAL`

- **Case 4 其他错误**
  - 返回 -1
  - `errno` 设置为对应错误码（`EMFILE`、`ENFILE` 等）

#### 系统算法

```
pipe2(fd, flag):
  if !flag: return pipe(fd)                              // flag=0 直接委托 pipe

  ret = __syscall(SYS_pipe2, fd, flag)                    // 尝试原子的 pipe2
  if ret != -ENOSYS: return __syscall_ret(ret)             // 成功或明确失败

  // SYS_pipe2 不可用时的回退方案
  if flag & ~(O_CLOEXEC|O_NONBLOCK):                      // 只支持这两种标志
    return __syscall_ret(-EINVAL)

  ret = pipe(fd)                                           // 先创建管道
  if ret: return ret

  if flag & O_CLOEXEC:
    __syscall(SYS_fcntl, fd[0], F_SETFD, FD_CLOEXEC)      // 手动设置 close-on-exec
    __syscall(SYS_fcntl, fd[1], F_SETFD, FD_CLOEXEC)

  if flag & O_NONBLOCK:
    __syscall(SYS_fcntl, fd[0], F_SETFL, O_NONBLOCK)      // 手动设置非阻塞
    __syscall(SYS_fcntl, fd[1], F_SETFL, O_NONBLOCK)

  return 0
```

#### 依赖

- `SYS_pipe2` — Linux 内核系统调用（原子操作，Linux 2.6.27+）
- `pipe(fd)` — 回退方案中的管道创建
- `SYS_fcntl` + `F_SETFD` / `F_SETFL` — 手动设置描述符标志
- `O_CLOEXEC`, `O_NONBLOCK`, `FD_CLOEXEC` — 文件描述符标志
