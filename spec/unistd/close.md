# close.c 规约

> musl libc POSIX 文件描述符关闭系统调用封装。`close` 在 `<unistd.h>` 中声明。

---

## 依赖图

```
close (Public)
  ├── __aio_close(fd) — AIO 关闭回调（弱别名，默认 dummy 函数原样返回 fd）
  └── __syscall_cp(SYS_close, ...)
        └── __syscall_ret(...)
  └── dummy (static) — 默认 AIO 关闭回调，原样返回 fd

weak_alias(dummy, __aio_close)
```

---

## 函数规约

### close

```c
int close(int fd);
```

[Visibility]: User — `<unistd.h>` POSIX 标准函数，用户程序可直接调用

#### Intent

关闭文件描述符 `fd`，释放与之关联的内核资源。在关闭前调用 AIO 关闭回调，并特殊处理 `EINTR` 错误（POSIX 要求 close 在 `EINTR` 时不重试，但 Linux 内核的实际行为是重试后成功）。

#### 前置条件

- `fd`: 有效的已打开文件描述符

#### 后置条件

- **Case 1 成功关闭**
  - `fd` 不再可用
  - 释放文件描述符及关联的内核数据结构
  - 返回 0

- **Case 2 `fd` 无效**
  - 返回 -1
  - `errno` 设置为 `EBADF`

- **Case 3 被 `EINTR` 中断**
  - musl 将 `EINTR` 视为成功，返回 0
  - 这符合 Linux 内核语义：当 close 返回 `EINTR` 时，fd 实际上已被释放

#### 系统算法

```
close(fd):
  fd = __aio_close(fd)          // 1. AIO 关闭回调（默认无操作）
  r = __syscall_cp(SYS_close, fd) // 2. 执行内核 close 系统调用
  if r == -EINTR: r = 0          // 3. EINTR 视为成功
  return __syscall_ret(r)        // 4. 转换为 libc 约定
```

#### 依赖

- `SYS_close` — Linux 内核系统调用编号 (x86_64: 3, aarch64: 57)
- `__aio_close` — AIO 关闭回调（若未链接 AIO 实现则为弱别名 dummy 函数）

---

### dummy (static)

```c
static int dummy(int fd) { return fd; }
```

[Visibility]: Internal (不导出) — 仅作为 `__aio_close` 的默认弱别名实现

#### Intent

提供 AIO 关闭回调的默认实现：不做任何操作，原样返回文件描述符。
