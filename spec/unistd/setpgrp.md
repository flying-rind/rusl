# setpgrp.c 规约

> musl libc BSD/XOPEN 进程组设置接口。`setpgrp` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
setpgrp (Public)
  └── setpgid(0, 0) — 同模块内部依赖
        └── syscall(SYS_setpgid, 0, 0)
              ├── __syscall(SYS_setpgid, 0, 0) — 原始系统调用
              └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### setpgrp

```c
pid_t setpgrp(void);
```

[Visibility]: User — `<unistd.h>` POSIX (XSI) / BSD 兼容函数，用户程序可直接调用

#### Intent

将当前调用进程的进程组 ID 设置为自身 PID，等价于创建以自身为首进程的新进程组。该函数是 `setpgid(0, 0)` 的 BSD/XOPEN 便捷封装：将当前进程的 PGID 设为其自身 PID，从而将当前进程放入一个新的进程组（以自身为首进程）。

#### 前置条件

- 调用进程未执行过 `execve`（否则子进程不允许修改自身进程组）
- 调用进程当前不在以其自身为首进程的进程组中
- 进程必须为其所在会话的成员

#### 后置条件

- **Case 1 成功**
  - 当前进程的进程组 ID 被设置为当前进程的 PID
  - 当前进程成为新进程组的首进程
  - 返回新的进程组 ID（即当前进程的 PID）

- **Case 2 失败**
  - 返回 -1
  - `errno` 设置为对应错误码（如 `EPERM` — 调用进程是会话首进程且已在一个不同进程组中；或当前进程已执行过 `execve`）

#### 系统算法

```
setpgrp():
  return setpgid(0, 0)
```

即：
1. 调用 `setpgid(0, 0)`（同模块内 `setpgid.c` 实现的 POSIX 函数）
2. 参数 `pid=0` 表示当前进程，`pgid=0` 表示使用当前进程的 PID 作为新进程组 ID
3. 内部通过 `syscall(SYS_setpgid, 0, 0)` 执行内核系统调用

#### 与 setpgid 的关系

```
setpgrp() ≡ setpgid(0, 0)
```

`setpgrp(void)` 是 BSD/XOPEN 的历史遗留接口，语义精确等价于 `setpgid(0, 0)`。musl 在 `<unistd.h>` 中同时提供两者，`setpgrp` 直接调用 `setpgid` 实现。

#### 依赖

- `setpgid` — 同模块内部函数，定义在 `src/unistd/setpgid.c`（详见 setpgid.md）
- `SYS_setpgid` — Linux 内核系统调用编号 (x86_64: 109, aarch64: 154)
- `syscall` — 内部宏，定义在 `src/internal/syscall.h`
