# _exit.c 规约

> musl libc POSIX 进程立即终止函数。`_exit` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
_exit (Public)
  └── _Exit(status) — ISO C 标准函数，定义在 <stdlib.h>
```

---

## 函数规约

### _exit

```c
_Noreturn void _exit(int status);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

立即终止调用进程，不执行任何清理操作：
- 不调用 `atexit()` 或 `on_exit()` 注册的函数
- 不刷新 stdio 缓冲区
- 不删除临时文件
- 关闭所有打开的文件描述符（由内核完成）
- 子进程被 init（PID 1）收养

#### 前置条件

- `status`: 进程退出状态码。仅低 8 位对父进程可见（通过 `wait()` 系列函数）。`status & 0xFF` 为有效退出状态

#### 后置条件

- 调用进程终止（函数不返回）
- `status & 0xFF` 成为进程的退出状态
- 为子进程生成 `SIGCHLD` 信号（若父进程未设置 `SA_NOCLDWAIT`）
- 若父进程调用 `wait()`，将收到此退出状态

#### 系统算法

```
_exit(status):
  _Exit(status)              // 委托给 ISO C _Exit，它调用 SYS_exit_group 系统调用
```

musl 的 `_exit` 实现极为简单：直接委托给 ISO C 标准函数 `_Exit()`，后者最终调用 `SYS_exit_group` 系统调用终止整个线程组。

#### 依赖

- `_Exit(int)` — ISO C 标准函数，定义在 `<stdlib.h>`，实现于 `src/exit/_Exit.c`
- `SYS_exit_group` — Linux 内核系统调用（由 `_Exit` 内部调用）
