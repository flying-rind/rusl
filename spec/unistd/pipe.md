# pipe.c 规约

> musl libc POSIX 管道创建系统调用封装。`pipe` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
pipe (Public)
  ├── [SYS_pipe 存在时] syscall(SYS_pipe, fd)
  └── [SYS_pipe 不存在时] syscall(SYS_pipe2, fd, 0)
```

---

## 函数规约

### pipe

```c
int pipe(int fd[2]);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

创建一对单向管道文件描述符：`fd[0]` 用于读取，`fd[1]` 用于写入。写入 `fd[1]` 的数据可以通过 `fd[0]` 读取，实现内核缓冲的单向数据流。

#### 前置条件

- `fd`: 指向至少 2 个 `int` 数组的非空指针

#### 后置条件

- **Case 1 成功创建**
  - `fd[0]` 设置为管道读端文件描述符
  - `fd[1]` 设置为管道写端文件描述符
  - 返回 0

- **Case 2 失败**
  - `fd` 数组内容不变
  - 返回 -1
  - `errno` 设置为 `EMFILE`（进程 fd 达上限）或 `ENFILE`（系统 fd 达上限）

#### 系统算法

```
pipe(fd):
  #ifdef SYS_pipe:
    return syscall(SYS_pipe, fd)       // 直接使用 pipe 系统调用
  #else:
    return syscall(SYS_pipe2, fd, 0)   // 回退到 pipe2(fd, 0)
  #endif
```

注意：某些架构（mips、sh）有手写汇编实现的 `pipe` 函数，因为内核 pipe 返回两个值（`r0=fd[0]`, `r1=fd[1]`），需要特殊处理。

#### 依赖

- `SYS_pipe` — Linux 内核系统调用（某些架构）
- `SYS_pipe2` — Linux 内核 pipe2 系统调用（flags=0 等价于 pipe）
