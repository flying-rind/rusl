# getpgrp.c 规约

> musl libc POSIX 调用进程的进程组 ID 获取系统调用封装。`getpgrp` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getpgrp (Public)
  └── __syscall(SYS_getpgid, 0)
        — 以 pid=0（当前进程）调用 SYS_getpgid 获取自身进程组 ID
```

---

## 函数规约

### getpgrp

```c
pid_t getpgrp(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取当前调用进程的进程组 ID（PGID）。该函数是 `getpgid(0)` 的简化形式，等价于以参数 0 调用 `SYS_getpgid` 系统调用。使用 `__syscall` 直接返回内核原始值，因为查询自身进程组 ID 始终成功。

#### 前置条件

- 无前置条件。此函数在任意进程上下文中均可调用。

#### 后置条件

- **Case 1 始终成功**
  - 返回调用进程的进程组 ID（正整数）
  - 该值即创建进程组时的首进程 PID，或通过 `setpgid` 设置的进程组 ID
  - 永远不会失败，不会设置 `errno`

#### 系统算法

```
getpgrp():
  return __syscall(SYS_getpgid, 0)
```

即：
1. 以参数 `pid=0` 调用 `__syscall(SYS_getpgid, 0)` 执行内核系统调用
2. PID 为 0 时，内核返回调用进程自身的进程组 ID
3. 直接返回内核原始值，不经 `__syscall_ret` 转换

#### 与 getpgid 的关系

```
getpgrp() ≡ getpgid(0)
```

`getpgrp(void)` 是 POSIX 定义的无参版本，比 `getpgid(pid_t pid)` 更为简便。musl 实现中 `getpgrp` 直接使用 `__syscall` 而非 `syscall`，因为 `pid=0` 时系统调用不会失败，无需 `__syscall_ret` 的 errno 转换。

#### 依赖

- `SYS_getpgid` — Linux 内核系统调用编号 (x86_64: 121, aarch64: 155)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
