# tcgetpgrp.c 规约

> musl libc POSIX 终端前台进程组获取函数。`tcgetpgrp` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
tcgetpgrp (Public)
  └── ioctl(fd, TIOCGPGRP, &pgrp) — 设备控制操作，获取终端前台进程组
        └── syscall(SYS_ioctl, fd, TIOCGPGRP, &pgrp) — 内核 ioctl 系统调用
```

---

## 函数规约

### tcgetpgrp

```c
pid_t tcgetpgrp(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取与文件描述符 `fd` 关联的终端的前台进程组 ID。通过 `TIOCGPGRP` ioctl 控制命令实现。

#### 前置条件

- `fd`: 有效的已打开文件描述符，必须关联到一个会话的控制终端

#### 后置条件

- **Case 1 成功**
  - 返回与 `fd` 关联的终端的前台进程组 ID（`pid_t` 类型，非负整数）

- **Case 2 `fd` 不关联终端或 ioctl 失败**
  - 返回 -1
  - `errno` 由 `ioctl` 设置（如 `EBADF`、`ENOTTY`）

#### 系统算法

```
tcgetpgrp(fd):
  int pgrp                                     // 用于接收前台进程组 ID
  if ioctl(fd, TIOCGPGRP, &pgrp) < 0:          // 1. 执行 TIOCGPGRP ioctl
    return -1                                   // 2a. 失败返回 -1
  return pgrp                                   // 2b. 成功返回进程组 ID
```

即：
1. 声明 `int pgrp` 作为 ioctl 的输出缓冲区
2. 调用 `ioctl(fd, TIOCGPGRP, &pgrp)` — `TIOCGPGRP` 将终端的前台进程组 ID 写入 `pgrp`
3. 若 `ioctl` 返回负值（出错），函数返回 -1
4. 否则返回 `pgrp` 的值

#### 依赖

- `ioctl` — 设备控制操作（`src/misc/ioctl.c`）
- `TIOCGPGRP` — 获取前台进程组的 ioctl 请求码 (`0x540F`)，定义在 `<sys/ioctl.h>`（`bits/ioctl.h`）
- `pid_t` — 进程 ID 类型，定义在 `<sys/types.h>`
