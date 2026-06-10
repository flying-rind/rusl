# \_\_fdopen.c 规约

> musl libc 内部 `fdopen` 主实现。从已打开的文件描述符和 mode 字符串构造 `FILE` 流对象，分配内存、配置缓冲区、设置操作函数指针，并将流登记到全局打开文件链表中。

---

## 依赖图

```
__fdopen
  ├─> strchr               (<string.h>)
  ├─> malloc                (<stdlib.h>)
  ├─> memset                (<string.h>)
  ├─> __syscall(SYS_fcntl, ...)    (内核)
  ├─> __syscall(SYS_ioctl, ...)    (内核)
  ├─> __stdio_read          (see __stdio_read.c spec)
  ├─> __stdio_write         (see __stdio_write.c spec)
  ├─> __stdio_seek          (see __stdio_seek.c spec)
  ├─> __stdio_close         (see __stdio_close.c spec)
  ├─> __ofl_add             (see ofl_add.c spec)
  └─> libc.threaded         (libc.h)
```

---

## 函数规约

### 1. \_\_fdopen

```c
FILE *__fdopen(int fd, const char *mode);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。通过 `weak_alias(__fdopen, fdopen)` 作为 POSIX 标准函数 `fdopen` 暴露给用户。

#### Intent

将已有的文件描述符封装为 `FILE` 流，根据 mode 字符串配置流的读写权限、缓冲策略和操作函数指针。主要功能：
1. 验证 mode 字符串格式
2. 分配 `FILE` 结构体 + 缓冲区（`sizeof(FILE) + UNGET + BUFSIZ`）
3. 根据 mode 设置读写限制标志（`F_NOWR`/`F_NORD`）
4. 根据 mode 设置追加模式、close-on-exec
5. 检测终端以决定行缓冲模式
6. 设置操作函数指针（`read`/`write`/`seek`/`close`）
7. 注册到全局打开文件链表

#### 前置条件

- `fd`: 有效的已打开文件描述符
- `mode`: 非 NULL 的合法 mode 字符串（首字符为 `'r'`、`'w'` 或 `'a'`）

#### 后置条件

**Case 1: 成功**

- 在堆上分配了 `sizeof(FILE) + UNGET + BUFSIZ` 字节的内存
- `f->fd = fd`
- `f->buf` 指向紧接 `FILE` 结构体之后（偏移 `sizeof(FILE) + UNGET`）
- `f->buf_size = BUFSIZ`
- 根据 mode 设置了 `f->flags`（读写限制、追加等）
- `f->lbf = '\n'`（若为可写终端） 或 `EOF`（否则）
- 所有操作函数指针已设置：
  - `f->read = __stdio_read`
  - `f->write = __stdio_write`
  - `f->seek = __stdio_seek`
  - `f->close = __stdio_close`
- `f->lock = -1`（若 libc 为单线程模式）
- `f` 已加入全局打开文件链表（`__ofl_add`）
- 返回 `f`

**Case 2: 失败**

- 若 mode 首字符无效：设置 `errno = EINVAL`，返回 `NULL`（`0`）
- 若内存分配失败：返回 `NULL`（`0`），errno 由 `malloc` 设置

#### 系统算法

```
__fdopen(fd, mode):
  /* 1. 验证 mode 首字符 */
  if !strchr("rwa", *mode):
    errno = EINVAL
    return 0

  /* 2. 分配 FILE + 缓冲区 */
  f = malloc(sizeof *f + UNGET + BUFSIZ)
  if !f:
    return 0

  /* 3. 仅清零结构体，不清零缓冲区 */
  memset(f, 0, sizeof *f)

  /* 4. 读写限制 */
  if !strchr(mode, '+'):
    f->flags = (*mode == 'r') ? F_NOWR : F_NORD

  /* 5. close-on-exec */
  if strchr(mode, 'e'):
    __syscall(SYS_fcntl, fd, F_SETFD, FD_CLOEXEC)

  /* 6. 追加模式 */
  if *mode == 'a':
    flags = __syscall(SYS_fcntl, fd, F_GETFL)
    if !(flags & O_APPEND):
      __syscall(SYS_fcntl, fd, F_SETFL, flags | O_APPEND)
    f->flags |= F_APP

  /* 7. 设置 fd 和缓冲区 */
  f->fd = fd
  f->buf = (unsigned char *)f + sizeof *f + UNGET
  f->buf_size = BUFSIZ

  /* 8. 终端检测 -> 行缓冲 */
  f->lbf = EOF
  if !(f->flags & F_NOWR) && __syscall(SYS_ioctl, fd, TIOCGWINSZ, &wsz) == 0:
    f->lbf = '\n'

  /* 9. 设置操作函数指针 */
  f->read = __stdio_read
  f->write = __stdio_write
  f->seek = __stdio_seek
  f->close = __stdio_close

  /* 10. 单线程模式：预设 lock = -1 */
  if !libc.threaded:
    f->lock = -1

  /* 11. 加入全局打开文件链表 */
  return __ofl_add(f)
```

#### 不变量

- `f->buf` 总位于 `(unsigned char*)f + sizeof(*f) + UNGET`，即 FILE 结构体后 8 字节开始
- `UNGET`（8 字节）为 `ungetc` 保留空间，位于 FIL结构体和写缓冲区之间
- `f->lock = -1` 仅在 `!libc.threaded` 时设置；多线程模式下 `f->lock = 0`（由 `memset` 确何）

#### 依赖

- `strchr()` — 字符查找（`<string.h>`）
- `malloc()` — 动态内存分配（`<stdlib.h>`）
- `memset()` — 内存填充（`<string.h>`）
- `__syscall(SYS_fcntl, ...)` — fcntl 系统调用（内核接口）
- `__syscall(SYS_ioctl, ...)` — ioctl 系统调用（内核接口）
- `__stdio_read()` — 默认读操作（本模块）
- `__stdio_write()` — 默认写操作（本模块）
- `__stdio_seek()` — 默认定位操作（本模块）
- `__stdio_close()` — 默认关闭操作（本模块）
- `__ofl_add()` — 加入全局文件链表（本模块，see `ofl_add.c` spec）
- `libc` — 全局运行时状态（`libc.h`）
- `EINVAL` — 错误码（`<errno.h>`）
- `UNGET`, `BUFSIZ`, `F_NOWR`, `F_NORD`, `F_APP` — 常量宏（`stdio_impl.h`）
- `O_APPEND`, `FD_CLOEXEC`, `F_SETFD`, `F_SETFL`, `F_GETFL` — 系统调用标志（`<fcntl.h>`）
- `TIOCGWINSZ`, `struct winsize` — 终端窗口大小（`<sys/ioctl.h>`）
- `EOF` — 文件结束标志（`<stdio.h>`）
