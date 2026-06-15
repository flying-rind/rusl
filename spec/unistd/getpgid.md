# getpgid.c 规约

> musl libc POSIX 进程组 ID 获取系统调用封装。`getpgid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getpgid (Public)
  └── syscall(SYS_getpgid, pid)
        ├── __syscall(SYS_getpgid, pid) — 原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### getpgid

```c
pid_t getpgid(pid_t pid);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取指定进程 `pid` 的进程组 ID（PGID）。若 `pid` 为 0，则获取调用进程自身的进程组 ID。该调用是 `SYS_getpgid` 系统调用的薄封装，使用 `syscall` 宏进行 `__syscall_ret` 返回值转换以正确设置 `errno`。

#### 前置条件

- `pid`: 有效的进程 ID，或 0（表示当前进程）

#### 后置条件

- **Case 1 成功**
  - 返回指定进程的进程组 ID（正整数）
  - 若 `pid` 为 0，返回调用进程自身的进程组 ID

- **Case 2 `pid` 对应的进程不存在**
  - 返回 -1
  - `errno` 设置为 `ESRCH`

- **Case 3 无权限访问指定进程**
  - 返回 -1
  - `errno` 设置为 `EPERM`

#### 系统算法

```
getpgid(pid):
  return syscall(SYS_getpgid, pid)
```

即：
1. 调用 `__syscall(SYS_getpgid, pid)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将内核返回值转换为 libc 约定（错误时设置 errno 返回 -1）

#### 依赖

- `SYS_getpgid` — Linux 内核系统调用编号 (x86_64: 121, aarch64: 155)
- `syscall` — 内部宏 `__syscall_ret(__syscall(...))`，定义在 `src/internal/syscall.h`
- `__syscall_ret` — 返回值转换，定义在 `src/internal/syscall_ret.c`
