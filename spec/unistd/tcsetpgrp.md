# tcsetpgrp.c 规约

> musl libc POSIX 终端前台进程组设置函数。`tcsetpgrp` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
tcsetpgrp (Public)
  └── ioctl(fd, TIOCSPGRP, &pgrp_int) — 设备控制操作，设置终端前台进程组
        └── syscall(SYS_ioctl, fd, TIOCSPGRP, &pgrp_int) — 内核 ioctl 系统调用
```

---

## 函数规约

### tcsetpgrp

```c
int tcsetpgrp(int fd, pid_t pgrp);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将文件描述符 `fd` 关联的终端的前台进程组设置为 `pgrp`。通过 `TIOCSPGRP` ioctl 控制命令实现。调用进程必须与终端属于同一会话。

#### 前置条件

- `fd`: 有效的已打开文件描述符，必须关联到调用进程所在会话的控制终端
- `pgrp`: 目标前台进程组 ID，必须与调用进程属于同一会话
- 调用进程必须持有终端的控制权

#### 后置条件

- **Case 1 成功**
  - 终端的前台进程组被设置为 `pgrp`
  - 返回 0

- **Case 2 失败**
  - 返回 -1
  - `errno` 由 `ioctl` 设置（如 `EBADF`、`ENOTTY`、`EPERM`、`EINVAL`）

#### 系统算法

```
tcsetpgrp(fd, pgrp):
  int pgrp_int = pgrp                         // 1. 将 pid_t 转为 int（ioctl 参数要求）
  return ioctl(fd, TIOCSPGRP, &pgrp_int)      // 2. 执行 TIOCSPGRP ioctl
```

即：
1. 将 `pid_t` 类型的 `pgrp` 赋值给 `int pgrp_int`，以满足 ioctl 的参数类型要求
2. 调用 `ioctl(fd, TIOCSPGRP, &pgrp_int)` — `TIOCSPGRP` 将终端的前台进程组设置为 `pgrp_int` 的值
3. 直接返回 ioctl 的结果（成功 0，失败 -1）

#### 依赖

- `ioctl` — 设备控制操作（`src/misc/ioctl.c`）
- `TIOCSPGRP` — 设置前台进程组的 ioctl 请求码 (`0x5410`)，定义在 `<sys/ioctl.h>`（`bits/ioctl.h`）
- `pid_t` — 进程 ID 类型，定义在 `<sys/types.h>`
