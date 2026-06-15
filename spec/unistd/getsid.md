# getsid.c 规约

> musl libc POSIX 会话 ID 获取系统调用封装。`getsid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getsid (Public)
  └── syscall(SYS_getsid, pid)
        ├── __syscall(SYS_getsid, pid) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### getsid

```c
pid_t getsid(pid_t pid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取指定进程 `pid` 的会话 ID（Session ID）。会话 ID 是创建该会话的进程（会话首进程）的 PID。若 `pid` 为 0，则获取调用进程自身的会话 ID。该调用是 `SYS_getsid` 系统调用的薄封装，使用 `syscall` 宏进行 `__syscall_ret` 返回值转换以正确设置 `errno`。

#### 前置条件

- `pid`: 有效的进程 ID，或 0（表示当前进程）

#### 后置条件

- **Case 1 成功**
  - 返回指定进程的会话 ID（正整数，等于会话首进程的 PID）
  - 若 `pid` 为 0，返回调用进程自身的会话 ID

- **Case 2 `pid` 对应的进程不存在**
  - 返回 -1
  - `errno` 设置为 `ESRCH`

- **Case 3 无权限访问指定进程**
  - 返回 -1
  - `errno` 设置为 `EPERM`

#### 系统算法

```
getsid(pid):
  return syscall(SYS_getsid, pid)
```

即：
1. 调用 `__syscall(SYS_getsid, pid)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 依赖

- `SYS_getsid` — Linux 内核系统调用编号 (x86_64: 124, aarch64: 156)
- `syscall` — 内部宏 `__syscall_ret(__syscall(...))`，定义在 `src/internal/syscall.h`
- `__syscall_ret` — 返回值转换，定义在 `src/internal/syscall_ret.c`
