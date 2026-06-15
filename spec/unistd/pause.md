# pause.c 规约

> musl libc POSIX 信号等待函数。`pause` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
pause (Public)
  └── sys_pause_cp() — 内部封装：可被信号取消的 pause 系统调用
        ├── __syscall_cp(SYS_pause) — 可被信号取消的原始系统调用
        └── __syscall_ret(...) — 返回值转换为 libc 约定
```

---

## 函数规约

### pause

```c
int pause(void);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

阻塞调用进程，直到收到一个信号并被信号处理器捕获，或者进程被该信号终止。`pause` 仅在信号处理器返回后才返回，此时返回值为 -1 且 `errno` 被设置为 `EINTR`。

该实现使用 `sys_pause_cp`（可被信号取消的内部分装），使 `pause` 成为一个取消点（cancellation point），允许线程在被 `pthread_cancel` 取消时安全退出。

#### 前置条件

- 无

#### 后置条件

- **Case 1 被信号处理器中断（正常情况）**
  - 信号处理器已被调用并返回
  - 返回 -1
  - `errno` 设置为 `EINTR`

- **Case 2 被信号终止**
  - 进程终止（函数不返回）

#### 系统算法

```
pause():
  return sys_pause_cp()        // 调用可被信号取消的 pause 系统调用
```

musl 实现极为简单：直接委托给内部封装 `sys_pause_cp()`，该封装等价于：
1. 调用 `__syscall_cp(SYS_pause)` 执行内核系统调用
2. 通过 `__syscall_ret()` 将返回值转换为 libc 约定

在 Linux 上，`SYS_pause` 的行为是：内核挂起调用进程，直到收到一个信号并被信号处理器捕获，然后返回 `-EINTR`。

#### 依赖

- `sys_pause_cp()` — 内部封装，定义在 `src/internal/` 中（通过 `src/internal/syscall.h`）
- `SYS_pause` — Linux 内核系统调用编号 (x86_64: 34, aarch64: 1061)
