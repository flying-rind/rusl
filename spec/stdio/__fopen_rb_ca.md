# \_\_fopen_rb_ca.c 规约

> musl libc 内部调用方分配 FILE（Caller-Allocated）的只读打开实现。由调用方提供 `FILE` 结构体内存和缓冲区，以只读方式打开文件，并设置必要的流操作函数指针。用于实现 `freopen` 等需要复用 `FILE` 内存的场景。

---

## 依赖图

```
__fopen_rb_ca
  ├─> memset          (<string.h>)
  ├─> sys_open        (内核, via __sys_open 宏)
  ├─> __syscall(SYS_fcntl, F_SETFD, FD_CLOEXEC)   (内核)
  ├─> __stdio_read    (see __stdio_read.c spec)
  ├─> __stdio_seek    (see __stdio_seek.c spec)
  └─> __stdio_close   (see __stdio_close.c spec)
```

---

## 函数规约

### 1. \_\_fopen_rb_ca

```c
FILE *__fopen_rb_ca(const char *filename, FILE *f, unsigned char *buf, size_t len);
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。与 `__fclose_ca` 配套，供 `freopen` 等复用 `FILE` 内存的内部实现调用。

#### Intent

以只读方式打开文件，使用调用方提供的 `FILE` 内存和缓冲区。文件以 `O_RDONLY|O_CLOEXEC` 打开，并设置 close-on-exec 标志（双重保险）。为 `f` 设置只读操作函数指针（`read`、`seek`、`close`），不设置 `write` 函数指针。

#### 前置条件

- `filename`: 非 NULL 的文件路径字符串，以 null 结尾
- `f`: `FILE*`，调用方提供的 `FILE` 内存（有效地址）
- `buf`: `unsigned char*`，调用方提供的缓冲区，长度至少为 `len`
- `len`: 缓冲区总长度，必须大于 `UNGET`（通常为 `BUFSIZ + UNGET`）

#### 后置条件

**Case 1: 成功打开文件**

- `f` 的全部字段首先被 `memset` 清零
- `f->fd` = 打开的文件描述符（`>= 0`，`O_RDONLY|O_CLOEXEC`）
- `f->fd` 的 `FD_CLOEXEC` 已通过 `fcntl(F_SETFD)` 确认设置
- `f->flags = F_NOWR | F_PERM`（禁止写 + 永久流标志）
- `f->buf = buf + UNGET`（为 ungetc 预留 8 字节）
- `f->buf_size = len - UNGET`（可用缓冲区大小）
- `f->read = __stdio_read`
- `f->seek = __stdio_seek`
- `f->close = __stdio_close`
- `f->lock = -1`（初始无锁，单线程模式）
- 返回 `f`（指向调用方提供的 `FILE`）

**Case 2: 打开文件失败**

- `f` 已被 `memset` 清零（字段被覆盖但有效）
- `f->fd = -1`（由 `sys_open` 设置的错误返回值）
- 返回 `NULL`（`0`）

#### 系统算法

```
__fopen_rb_ca(filename, f, buf, len):
  /* 1. 清零 FILE 结构体 */
  memset(f, 0, sizeof *f)

  /* 2. 打开文件 */
  f->fd = sys_open(filename, O_RDONLY | O_CLOEXEC)
  if f->fd < 0:
    return 0

  /* 3. 双重确认 close-on-exec */
  __syscall(SYS_fcntl, f->fd, F_SETFD, FD_CLOEXEC)

  /* 4. 设置流标志与缓冲区 */
  f->flags = F_NOWR | F_PERM
  f->buf = buf + UNGET        // 预留 UNGET 字节
  f->buf_size = len - UNGET

  /* 5. 设置操作函数指针（只读，不设 write） */
  f->read = __stdio_read
  f->seek = __stdio_seek
  f->close = __stdio_close
  f->lock = -1                 // 单线程模式

  return f
```

#### 不变量

- `f->flags` 始终包含 `F_NOWR`（禁止写入）
- `f->buf` 始终为 `buf + UNGET`（内部缓冲区前预留反推空间）
- `f->lock = -1` 表示初始无锁，`FLOCK` 宏将此视为无需加锁
- 不设置 `f->write` 函数指针（读打开流不支持写操作）

#### 依赖

- `memset()` — 内存填充（`<string.h>`，libc 标准函数）
- `sys_open()` — 打开文件系统调用宏（`syscall.h`，最终调用 `SYS_openat` 或 `SYS_open`）
- `__syscall(SYS_fcntl, ...)` — fcntl 系统调用（内核接口）
- `__stdio_read()` — 默认读操作（本模块，see `__stdio_read.c` spec）
- `__stdio_seek()` — 默认定位操作（本模块，see `__stdio_seek.c` spec）
- `__stdio_close()` — 默认关闭操作（本模块，see `__stdio_close.c` spec）
- `UNGET`, `F_NOWR`, `F_PERM` — 常量宏（`stdio_impl.h`）
- `O_RDONLY`, `O_CLOEXEC`, `FD_CLOEXEC`, `F_SETFD` — 系统调用标志（`<fcntl.h>`）
