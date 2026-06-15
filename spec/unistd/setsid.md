# setsid.c 规约

> musl libc POSIX 创建新会话系统调用封装。`setsid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setsid (Public)
  └── syscall(SYS_setsid)
        ├── __syscall(SYS_setsid) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### setsid

```c
pid_t setsid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

创建一个新会话（session）并将调用进程设为该会话的首进程。同时，调用进程成为新进程组的首进程，并且与之前的控制终端（controlling terminal）断开连接。这一操作是实现守护进程（daemon）化的关键步骤之一。该调用是 `SYS_setsid` 系统调用的薄封装。

#### 前置条件

- 调用进程不能已经是某个进程组的首进程（否则操作无意义，内核会拒绝）
- 进程必须具有创建新会话的权限（普通用户进程通常具备）

#### 后置条件

- **Case 1 成功**
  - 调用进程成为一个新会话的会话首进程
  - 调用进程成为一个新进程组的进程组首进程
  - 调用进程不再拥有控制终端
  - 返回新创建的会话 ID（等于调用进程的 PID）

- **Case 2 调用进程已是进程组首进程**
  - 返回 -1
  - `errno` 设置为 `EPERM`

#### 系统算法

```
setsid():
  return syscall(SYS_setsid)
```

即：
1. 调用 `__syscall(SYS_setsid)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 不变量

- 会话首进程同时也是其所在进程组的首进程
- 一个会话最多拥有一个控制终端
- 会话首进程退出时，会话中所有进程会收到 `SIGHUP` 信号

#### 依赖

- `SYS_setsid` — Linux 内核系统调用编号 (x86_64: 112, aarch64: 157)
- `syscall` — 内部宏 `__syscall_ret(__syscall(...))`，定义在 `src/internal/syscall.h`
- `__syscall_ret` — 返回值转换，定义在 `src/internal/syscall_ret.c`
