# getppid.c 规约

> musl libc POSIX 父进程 ID 获取系统调用封装。`getppid` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
getppid (Public)
  └── __syscall(SYS_getppid)
```

---

## 函数规约

### getppid

```c
pid_t getppid(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

获取当前调用进程的父进程 ID（PPID）。该调用是 `SYS_getppid` 系统调用的薄封装，使用 `__syscall` 直接返回内核原始返回值，因为此系统调用始终成功。

#### 前置条件

- 无前置条件。此函数在任意进程上下文中均可调用。

#### 后置条件

- **Case 1 始终成功**
  - 返回调用进程的父进程 ID（正整数）
  - 父进程退出后，该值可能变为 1（init 进程）
  - 永远不会失败，不会设置 `errno`

#### 系统算法

```
getppid():
  return __syscall(SYS_getppid)
```

即：
1. 调用 `__syscall(SYS_getppid)` 执行内核系统调用
2. 直接返回内核原始值（父进程 PPID），不经 `__syscall_ret` 转换

#### 依赖

- `SYS_getppid` — Linux 内核系统调用编号 (x86_64: 110, aarch64: 173)
- `__syscall` — 内部宏，定义在 `src/internal/syscall.h`
