# popen.c 规约

> musl libc 标准库管道执行函数实现。启动一个子进程执行 shell 命令，并返回一个 FILE 流以读写其标准输入/输出。

---

## 依赖图

```
popen (Public)
  ├── pipe2(p, O_CLOEXEC)                          — 创建带有 close-on-exec 标志的管道 (unistd.h)
  ├── fdopen(p[op], mode)                          — 从 fd 创建 FILE (src/stdio/fdopen.c)
  ├── __syscall(SYS_close, p[0])                   — 关闭管道 fd (syscall.h, 错误路径)
  ├── __syscall(SYS_close, p[1])                   — 关闭管道 fd (syscall.h, 错误路径)
  ├── __ofl_lock() / __ofl_unlock()                — 全局打开文件链表锁 (src/stdio/ofl.c)
  ├── posix_spawn_file_actions_init(&fa)           — 初始化 spawn 文件操作 (<spawn.h>)
  ├── posix_spawn_file_actions_addclose(&fa, fd)   — 添加 close 操作 (<spawn.h>)
  ├── posix_spawn_file_actions_adddup2(&fa, ...)   — 添加 dup2 操作 (<spawn.h>)
  ├── posix_spawn_file_actions_destroy(&fa)        — 销毁文件操作 (<spawn.h>)
  ├── posix_spawn(&pid, "/bin/sh", &fa, 0, ...)    — 启动子进程 (<spawn.h>)
  ├── strchr(mode, 'e')                            — 检查 'e' 标志 (<string.h>)
  ├── fcntl(p[op], F_SETFD, 0)                     — 清除 close-on-exec 标志 (<fcntl.h>)
  ├── fclose(f)                                    — 关闭 FILE (错误路径, src/stdio/fclose.c)
  └── __environ                                     — 环境变量指针数组 (全局变量)
```

---

## 函数规约

### 1. popen

```c
FILE *popen(const char *cmd, const char *mode);
```

[Visibility]: User — `<stdio.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

创建管道、fork 子进程，子进程执行 `/bin/sh -c <cmd>`，父进程获得与子进程的标准输入（`mode="w"`）或标准输出（`mode="r"`）相连的 `FILE*`。

关键实现细节：
1. 使用 `posix_spawn`（而非 `fork+exec`）启动子进程，精细控制文件描述符继承
2. 在 `posix_spawn` 之前，遍历全局打开文件链表（`__ofl_lock`），对所有带有 `pipe_pid` 的 FILE（即其他通过 popen 打开的流）添加 `close-on-exec` 操作，防止子进程继承不需要的管道文件描述符
3. 管道使用 `pipe2(O_CLOEXEC)` 创建，确保 exec 时自动关闭不使用的管道端

#### 前置条件

- `cmd`: 非空指针，指向以 `\0` 结尾的有效 shell 命令字符串
- `mode`: 非空指针，必须以 `'r'`（读子进程输出）或 `'w'`（写子进程输入）开头；可选后跟 `'e'`（表示 close-on-exec）
- `/bin/sh` 可执行文件存在
- 系统有足够的进程资源和文件描述符

#### 后置条件

- **Case 1 成功**
  - 创建子进程执行 `/bin/sh -c cmd`
  - `mode="r"` 时：返回的 `FILE*` 连接子进程 stdout（`p[0]` = 读端）；子进程的 `stdin=1`（`1-op=1`）被 dup 到写端 `p[1]`
  - `mode="w"` 时：返回的 `FILE*` 连接子进程 stdin（`p[1]` = 写端）；子进程的 `stdin=0`（`1-op=0`）被 dup 到读端 `p[0]`
  - 返回的 `FILE*` 的 `pipe_pid` 字段记录子进程 pid，供 `pclose` 使用
  - 若不包含 `'e'` 标志，返回描述符的 close-on-exec 被清除
  - 不使用的管道端 `p[1-op]` 被关闭

- **Case 2 失败**
  - 返回 `NULL`
  - `errno` 反映失败原因（可能为 `EINVAL`、`ENOMEM` 或系统调用错误码）

#### 系统算法

```
popen(cmd, mode):
  1. 模式解析:
     if *mode == 'r': op = 0 (父进程读)
     elif *mode == 'w': op = 1 (父进程写)
     else: errno = EINVAL; return NULL
  
  2. 创建管道:
     if pipe2(p, O_CLOEXEC) != 0: return NULL
     // p[0]=读端, p[1]=写端, 两端皆为 close-on-exec
  
  3. 从 fd 创建 FILE:
     f = fdopen(p[op], mode)
     if !f: 关闭 p[0], p[1]; return NULL
  
  4. posix_spawn 文件操作:
     e = ENOMEM
     if posix_spawn_file_actions_init(&fa) != 0: goto fail_fclose
     
     // 遍历所有打开的 FILE: 若任意 FILE 有 pipe_pid 设置，
     // 则在 spawn 子进程前关闭其 fd (防止泄漏给子进程)
     for (l in *__ofl_lock() 遍历):
       if l->pipe_pid:
         posix_spawn_file_actions_addclose(&fa, l->fd)  // 失败则 goto fail
  
     // dup2: 将子进程 stdin/stdout 重定向到管道的另一端
     // op=0(父读): dup2(p[1], 1) — 子进程 stdout 写入管道
     // op=1(父写): dup2(p[0], 0) — 子进程 stdin 从管道读取
     if posix_spawn_file_actions_adddup2(&fa, p[1-op], 1-op) != 0: goto fail
  
  5. 启动子进程:
     e = posix_spawn(&pid, "/bin/sh", &fa, 0,
                     (char*[]){"sh", "-c", (char*)cmd, NULL},
                     __environ)
     if e == 0:  // 成功
       销毁 fa
       f->pipe_pid = pid                    // 记录子进程 pid
       if !strchr(mode, 'e'):               // 若 mode 不含 'e'
         fcntl(p[op], F_SETFD, 0)           // 清除本端 close-on-exec
       关闭 p[1-op]                         // 关闭不使用的管道端
       解锁 __ofl_unlock()
       return f
  
  fail:
     解锁 __ofl_unlock()
     销毁 fa
  fail_fclose:
     fclose(f)        // 关闭 FILE，会自动 close fd
     关闭 p[1-op]     // 确保不使用的管道端也被关闭
     errno = e
     return NULL
```

#### 依赖

- `pipe2(int fds[2], int flags)` — 创建带标志的管道（来自 `<unistd.h>` / 系统调用）
- `fdopen(int fd, const char *mode)` — 从文件描述符创建 FILE（定义于 `src/stdio/__fdopen.c`）
- `fclose(FILE *f)` — 关闭 FILE（定义于 `src/stdio/fclose.c`）
- `fcntl(int fd, int cmd, ...)` — 文件描述符控制操作（来自 `<fcntl.h>`）
- `posix_spawn_file_actions_init/destroy/posix_spawn_file_actions_addclose/adddup2` — spawn 文件操作接口（来自 `<spawn.h>`）
- `posix_spawn(pid_t *pid, const char *path, const posix_spawn_file_actions_t *fa, const posix_spawnattr_t *attr, char *const argv[], char *const envp[])` — 创建子进程（来自 `<spawn.h>`）
- `strchr(const char *s, int c)` — 字符查找（来自 `<string.h>`）
- `__ofl_lock()` / `__ofl_unlock()` — 全局打开文件链表锁（定义于 `src/stdio/ofl.c`）
- `__environ` — 环境变量数组全局变量（声明于 `<unistd.h>`，定义于 `src/env/__environ.c`）
- `__syscall(SYS_close, fd)` — 关闭文件描述符系统调用（定义于 `src/internal/syscall.h`）
- `O_CLOEXEC` — close-on-exec 标志（来自 `<fcntl.h>`）
- `EINVAL` / `ENOMEM` — 错误码（来自 `<errno.h>`）
- `F_SETFD` — fcntl 设置描述符标志命令（来自 `<fcntl.h>`）
- `FILE` 结构体 `pipe_pid`、`fd`、`next` 字段（定义于 `src/internal/stdio_impl.h`）

#### 不变量

- 子进程始终通过 `/bin/sh -c` 执行，而非直接执行 `cmd`
- 所有 popen 打开的、有 `pipe_pid` 的管道 fd 都会在 spawn 新子进程时关闭，防止描述符泄漏
- 不使用 `fork`：使用 `posix_spawn` 实现更精细的文件描述符控制
