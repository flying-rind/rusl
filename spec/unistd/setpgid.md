# setpgid.c 规约

> musl libc POSIX 进程组 ID 设置系统调用封装。`setpgid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setpgid (Public)
  └── syscall(SYS_setpgid, pid, pgid)
        ├── __syscall(SYS_setpgid, pid, pgid) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### setpgid

```c
int setpgid(pid_t pid, pid_t pgid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

将指定进程 `pid` 的进程组 ID 设置为 `pgid`。若 `pid` 为 0，则操作调用进程自身。若 `pgid` 为 0，则将 `pid` 的值用作进程组 ID。此操作用于将进程移动到新的或已存在的进程组中，或创建新的进程组。该调用是 `SYS_setpgid` 系统调用的薄封装。

#### 前置条件

- `pid`: 有效的进程 ID，或 0（表示当前调用进程）
- `pgid`: 有效的进程组 ID，或 0（表示使用 `pid` 自身的值）
- 调用进程必须拥有适当的权限（与目标进程属于同一会话，或有 `CAP_SYS_ADMIN`）
- 目标进程与调用进程必须处于同一会话
- `pid` 不能是已执行过 `execve` 的子进程（防止竞争条件）

#### 后置条件

- **Case 1 成功**
  - 进程 `pid` 的进程组 ID 被设置为 `pgid`（若 `pgid` 为 0 则为 `pid` 自身）
  - 返回 0

- **Case 2 参数无效**
  - `pid` 对应的进程不存在，或不在同一会话
  - 返回 -1，`errno` 设置为 `ESRCH`

- **Case 3 权限不足**
  - 调用进程没有足够权限修改目标进程的进程组
  - 返回 -1，`errno` 设置为 `EPERM`

- **Case 4 进程组 ID 无效**
  - `pgid` 指定的进程组不存在且不与 `pid` 相同
  - 返回 -1，`errno` 设置为 `EPERM`

#### 系统算法

```
setpgid(pid, pgid):
  return syscall(SYS_setpgid, pid, pgid)
```

即：
1. 调用 `__syscall(SYS_setpgid, pid, pgid)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 依赖

- `SYS_setpgid` — Linux 内核系统调用编号 (x86_64: 109, aarch64: 154)
- `syscall` — 内部宏 `__syscall_ret(__syscall(...))`，定义在 `src/internal/syscall.h`
- `__syscall_ret` — 返回值转换，定义在 `src/internal/syscall_ret.c`
