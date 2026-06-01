# stdio_impl.h 规约

> **来源文件**: `musl/src/internal/stdio_impl.h`
> **复杂度层级**: Level 3 — 高度优化设计（完整 stdio 抽象 + 文件描述符管理层 + 线程安全锁）
> **依赖图**:
> ```
> syscall.h (系统调用宏)
>   -> struct _IO_FILE — musl 内部 FILE 结构体定义
>     -> 全局变量: __stdin_used, __stdout_used, __stderr_used
>       -> 锁操作: __lockfile(), __unlockfile()
>         -> 底层IO: __stdio_read(), __stdio_write(), __stdout_write(), __stdio_seek(), __stdio_close()
>           -> 读写切换: __toread(), __towrite()
>             -> 生命周期: __stdio_exit(), __stdio_exit_needed()
>               -> 公共接口: __overflow(), __uflow()  [protected visibility]
>                 -> fseek变体: __fseeko(), __fseeko_unlocked(), __ftello(), __ftello_unlocked()
>                   -> 写辅助: __fwritex(), __putc_unlocked()
>                     -> fdopen: __fdopen(), __fmodeflags()
>                       -> 打开文件列表: __ofl_add(), __ofl_lock(), __ofl_unlock()
>                         -> 线程安全: __register_locked_file(), __unlist_locked_file(), __do_orphaned_stdio_locks()
>                           -> misc: __getopt_msg(), __fopen_rb_ca(), __fclose_ca()
>                             -> 内联宏: feof, ferror, getc_unlocked, putc_unlocked, FLOCK/UNLOCK, FFINALLOCK
> ```

---

## 概述

`stdio_impl.h` 是 musl libc 标准 I/O 库的内部核心。它定义了 `struct _IO_FILE`（即 `FILE` 的真身）、所有内部缓冲 I/O 操作的函数接口，以及线程安全的文件锁管理机制。本文件是 musl stdio 子系统所有 `.c` 实现文件的公共基础。

**不变量 (Invariants)**：
- **I1**: 任何对 `FILE` 中 `rpos/rend/wpos/wend` 的修改必须在持有 `lock` 的状态下进行（多线程安全），或调用者保证单线程访问。
- **I2**: `buf` 和 `buf_size` 要么均为 0（无缓冲），要么描述一个有效缓冲区。`wbase` 指向写缓冲起始位置。
- **I3**: `prev/next` 构成全局打开文件双向链表（`ofl_head`），由 `__ofl_lock/__ofl_unlock` 保护。
- **I4**: `flags` 中的 `F_ERR` 和 `F_EOF` 一旦设置，不会被主动清除（除非显式调用 `clearerr`）。
- **I5**: `pipe_pid` 若为非零值，表示该 FILE 通过 `popen()` 创建，关闭时需调用 `waitpid` 回收子进程。

---

## 常量定义

### 缓冲区常量

```c
#define UNGET 8
```

**意图**: `ungetc` 推回缓冲区的大小（字节数）。在 musl 中，`ungetc` 支持最多 8 字节的推回。

---

### 文件状态标志 (flags)

```c
#define F_PERM   1    // 永久分配（不由 fclose 释放 FILE 结构自身）
#define F_NORD   4    // 不可读（流已关闭读取方向）
#define F_NOWR   8    // 不可写（流已关闭写入方向）
#define F_EOF   16    // 已遇到文件末尾
#define F_ERR   32    // 发生 I/O 错误
#define F_SVB   64    // 行缓冲（Line-buffered）
#define F_APP  128    // 追加模式 (O_APPEND)
```

**位掩码语义**:
| 标志 | 值 | 语义 | 修改场景 |
|------|-----|------|---------|
| `F_PERM` | 0x01 | 永久 FILE（如 stdin/stdout/stderr），fclose 不释放结构体 | `__fdopen` |
| `F_NORD` | 0x04 | 读方向已关闭（`f->read == NULL`，或 `shutdown(fd, SHUT_RD)` 后） | `fclose` / `freopen` |
| `F_NOWR` | 0x08 | 写方向已关闭 | `fclose` / `freopen` |
| `F_EOF` | 0x10 | 已读至文件末尾（`f->read()` 返回 0） | `__uflow` / `fread` |
| `F_ERR` | 0x20 | I/O 错误发生 | `__uflow` / `__overflow` |
| `F_SVB` | 0x40 | 行缓冲（`\n` 触发刷新） | `setvbuf` |
| `F_APP` | 0x80 | 追加模式（每次写入前 seek 到末尾） | `fopen("...a")` |

### 锁操作宏

```c
#define FFINALLOCK(f) ((f)->lock>=0 ? __lockfile((f)) : 0)
#define FLOCK(f) int __need_unlock = ((f)->lock>=0 ? __lockfile((f)) : 0)
#define FUNLOCK(f) do { if (__need_unlock) __unlockfile((f)); } while (0)
```

**意图**:
- `FFINALLOCK(f)`: 在函数入口处无条件获取 FILE 锁（仅在 `lock>=0` 时）。使用前需声明局部变量记录是否需要解锁。
- `FLOCK(f)`: 声明式加锁模式。声明 `int __need_unlock` 变量并加锁。
- `FUNLOCK(f)`: 对应的解锁宏。仅在 `__need_unlock` 为真时释放锁。

**注意**: `FLOCK`/`FUNLOCK` 模式利用了 C 语言中的宏+局部变量技巧，使得一个函数可以在入口加锁、出口自动解锁。锁变量 `lock` ≥ 0 表示是一个真实的 FILE 对象（需要锁保护），`lock < 0` 表示仅作为内存缓冲区使用（无需锁）。

---

### `MAYBE_WAITERS`

```c
#define MAYBE_WAITERS 0x40000000
```

**意图**: 在 FILE 的 `lock` 字段中标记"可能有等待者"。当 `__unlockfile` 发现 `lock` 为该值时，需执行 futex wake 操作唤醒等待线程。

---

## 结构体定义

### `struct _IO_FILE`

```c
struct _IO_FILE {
    unsigned flags;
    unsigned char *rpos, *rend;
    int (*close)(FILE *);
    unsigned char *wend, *wpos;
    unsigned char *mustbezero_1;
    unsigned char *wbase;
    size_t (*read)(FILE *, unsigned char *, size_t);
    size_t (*write)(FILE *, const unsigned char *, size_t);
    off_t (*seek)(FILE *, off_t, int);
    unsigned char *buf;
    size_t buf_size;
    FILE *prev, *next;
    int fd;
    int pipe_pid;
    long lockcount;
    int mode;
    volatile int lock;
    int lbf;
    void *cookie;
    off_t off;
    char *getln_buf;
    void *mustbezero_2;
    unsigned char *shend;
    off_t shlim, shcnt;
    FILE *prev_locked, *next_locked;
    struct __locale_struct *locale;
};
```

[Visibility]: Internal — musl 内部实现的 `FILE` 实际布局，POSIX 标准定义 `FILE` 为不透明类型

**字段分组语义**：

| 分组 | 字段 | 类型 | 语义 |
|------|------|------|------|
| **缓冲区读端** | `rpos` | `unsigned char *` | 当前读取位置 |
| | `rend` | `unsigned char *` | 读取缓冲区末尾（不可逾越） |
| **缓冲区写端** | `wpos` | `unsigned char *` | 当前写入位置 |
| | `wend` | `unsigned char *` | 写缓冲区末尾 |
| | `wbase` | `unsigned char *` | 写缓冲区起始（用于 flush 时追溯） |
| **总缓冲区** | `buf` | `unsigned char *` | 缓冲区起始地址（指向 `malloc` 分配或用户提供的缓冲区） |
| | `buf_size` | `size_t` | 缓冲区总大小（字节）；UNGET 模式下包含推回区域 |
| **虚函数表** | `close` | `int (*)(FILE *)` | 关闭操作函数指针 |
| | `read` | `size_t (*)(FILE *, unsigned char *, size_t)` | 底层读取操作 |
| | `write` | `size_t (*)(FILE *, const unsigned char *, size_t)` | 底层写入操作 |
| | `seek` | `off_t (*)(FILE *, off_t, int)` | 底层定位操作 |
| **文件描述符** | `fd` | `int` | 关联的文件描述符（无缓冲时为 -1） |
| **管道** | `pipe_pid` | `int` | popen 子进程 PID（非 popen 时为 0） |
| **锁** | `lock` | `volatile int` | 线程锁（<0 表示不需要锁定，0 表示未锁定，>0 表示已锁定） |
| | `lockcount` | `long` | 递归锁计数器 |
| **链表** | `prev` | `FILE *` | 全局打开文件链表的前驱（`ofl_head` 链表） |
| | `next` | `FILE *` | 全局打开文件链表的后继 |
| | `prev_locked` | `FILE *` | 线程持有的"锁定文件"链表前驱 |
| | `next_locked` | `FILE *` | 线程持有的"锁定文件"链表后继 |
| **标志/模式** | `flags` | `unsigned` | 文件状态位掩码（`F_*` 系列） |
| | `mode` | `int` | 文件打开模式（`O_RDONLY`/`O_WRONLY`/`O_RDWR` 等） |
| | `lbf` | `int` | 行缓冲标志：-1 表示行缓冲模式时的换行符（通常为 `'\n'`） |
| **扩展** | `cookie` | `void *` | 扩展数据（用于 `fopencookie` 等自定义流） |
| **偏移** | `off` | `off_t` | 逻辑文件偏移（用于缓冲与真实位置不一致时的修正） |
| **gets 缓冲区** | `getln_buf` | `char *` | `gets()`/`fgets()` 行缓冲区 |
| **扫描辅助** | `shend` | `unsigned char *` | 扫描结束位置（`shgetc` 使用，见 shgetc.h spec） |
| | `shlim` | `off_t` | 扫描宽度限制（`shgetc` 使用） |
| | `shcnt` | `off_t` | 已扫描字符计数 |
| **locale** | `locale` | `struct __locale_struct *` | 文件关联的 locale 设置（影响宽字符转换等） |
| **内部/哨兵** | `mustbezero_1` | `unsigned char *` | 必须为 NULL/0 的哨兵字段（用于检测结构体损坏） |
| | `mustbezero_2` | `void *` | 同上 |

---

## 全局变量声明

### 标准流指针

```c
extern hidden FILE *volatile __stdin_used;
extern hidden FILE *volatile __stdout_used;
extern hidden FILE *volatile __stderr_used;
```

**意图**: musl 内部实际使用的标准 I/O 流指针（非最终用户可见的 `stdin`/`stdout`/`stderr` 宏）。`volatile` 修饰用于防止编译器对多线程访问进行优化重排。

---

## 函数声明

### 锁操作

#### `int __lockfile(FILE *)`

```c
int __lockfile(FILE *);
```

[Visibility]: Internal — musl 内部 FILE 锁获取

**意图**: 获取 FILE 的互斥锁。若锁已被他人持有且 `lockcount == 0`，则进入 futex 等待。

**前置条件**: `f` 非 NULL，且 `f->lock >= 0`。

**后置条件**: 返回 0（成功获取锁），或返回非零（已持有锁/未加锁）。

#### `void __unlockfile(FILE *)`

```c
void __unlockfile(FILE *);
```

[Visibility]: Internal — musl 内部 FILE 锁释放

**意图**: 释放 FILE 的互斥锁。若有等待者则 futex wake。

---

### 底层 I/O 操作

#### `size_t __stdio_read(FILE *, unsigned char *, size_t)`

```c
size_t __stdio_read(FILE *, unsigned char *, size_t);
```

[Visibility]: Internal — musl 内部读缓冲区填充

**意图**: 从文件描述符读取数据填充 FILE 的缓冲区。实现缓冲区管理和 `readv` 系统调用。

**前置条件**: `f->fd >= 0`，`f->read == __stdio_read`。

**后置条件**: 返回实际读取的字节数（0 表示 EOF 或缓冲区满）。

#### `size_t __stdio_write(FILE *, const unsigned char *, size_t)`

```c
size_t __stdio_write(FILE *, const unsigned char *, size_t);
```

**意图**: 将数据写入文件描述符（缓冲或直写）。处理行缓冲、全缓冲、无缓冲三种模式的 I/O。

#### `size_t __stdout_write(FILE *, const unsigned char *, size_t)`

```c
size_t __stdout_write(FILE *, const unsigned char *, size_t);
```

**意图**: stdout 专用的写入函数。在发生写入错误时，设置 `F_ERR` 标志后**多次重试**（因为是 stdout，即使出错也应尽力输出）。

#### `off_t __stdio_seek(FILE *, off_t, int)`

```c
off_t __stdio_seek(FILE *, off_t, int);
```

**意图**: 底层定位操作。在 seek 前刷新写缓冲，并调整逻辑偏移 `f->off`。

#### `int __stdio_close(FILE *)`

```c
int __stdio_close(FILE *);
```

**意图**: 关闭文件描述符。若 FILE 由 `popen()` 创建（`pipe_pid != 0`），同时调用 `waitpid` 回收子进程。

---

### 读写模式切换

#### `int __toread(FILE *)` / `int __towrite(FILE *)`

```c
int __toread(FILE *);
int __towrite(FILE *);
```

**意图**: 将 FILE 从"空闲"或"写"模式切换到"读"模式（`__toread`），反之亦然（`__towrite`）。保证在 `fread` 后 `fwrite`（或反过来）时自动进行模式切换。

**前置条件**: `f` 非 NULL。切换方向必须是允许的（`F_NOWR` 未设置则可切换为写，`F_NORD` 未设置则可切换为读）。

---

### 生命周期管理

#### `void __stdio_exit(void)` / `void __stdio_exit_needed(void)`

```c
void __stdio_exit(void);
void __stdio_exit_needed(void);
```

**意图**: 
- `__stdio_exit()` — 进程退出时刷新并关闭所有打开的 FILE
- `__stdio_exit_needed()` — 通过 `atexit` 注册退出清理，确保 stdio 缓冲在 `exit()` 前被刷新

---

### 半公共接口 (protected visibility)

#### `int __overflow(FILE *, int)` / `int __uflow(FILE *)`

```c
int __overflow(FILE *, int);
int __uflow(FILE *);
```

[Visibility]: 半公共 — 使用 protected 可见性（`__attribute__((visibility("protected"))`），POSIX/C 标准未定义

**意图**:
- `__overflow(f, c)` — 将字符 `c` 写入 FILE 的缓冲区；若缓冲区满则触发刷新
- `__uflow(f)` — 从 FILE 的缓冲区读取一个字符；若缓冲区空则触发填充

**注意**: 使用 GCC 的 `protected` 可见性（而非 `hidden`），使得该符号在同一共享库内可直接调用（类似内部符号），但不能被外部库覆写。

---

### fseek 变体函数

| 函数 | 签名 | 意图 |
|------|------|------|
| `__fseeko` | `int __fseeko(FILE *, off_t, int)` | 带锁保护的 seek 操作 |
| `__fseeko_unlocked` | `int __fseeko_unlocked(FILE *, off_t, int)` | 无锁版本的 seek（调用者已持有锁） |
| `__ftello` | `off_t __ftello(FILE *)` | 带锁保护的 tell 操作 |
| `__ftello_unlocked` | `off_t __ftello_unlocked(FILE *)` | 无锁版本的 tell |

---

### 写入辅助函数

#### `size_t __fwritex(const unsigned char *, size_t, FILE *)`

**意图**: 无锁版本的 `fwrite` 核心实现。直接写数据到 FILE 的写缓冲区。

#### `int __putc_unlocked(int, FILE *)`

**意图**: 无锁版本的 `putc` 实现。将单字符写入 FILE 的写缓冲区。

---

### fdopen 相关

#### `FILE *__fdopen(int, const char *)`

**意图**: 将文件描述符包装为 `FILE *`。处理 `mode` 字符串解析和 FILE 结构体初始化。

#### `int __fmodeflags(const char *)`

**意图**: 将 `fopen` 模式字符串（`"r"`、`"w+"`、`"a"` 等）解析为 `open()` 的 flags 位掩码。

---

### 打开文件列表管理

#### `FILE *__ofl_add(FILE *)` / `FILE **__ofl_lock(void)` / `void __ofl_unlock(void)`

**意图**:
- `__ofl_add(f)` — 将 FILE 注册到全局打开文件链表
- `__ofl_lock()` — 获取打开文件链表锁，返回链表头指针的指针
- `__ofl_unlock()` — 释放打开文件链表锁

`__ofl_lock` 的返回值是 `FILE **`（指向链表头的指针），这使得调用方可以在持有锁时安全地遍历或修改链表。

---

### 线程安全的锁定文件跟踪

#### `void __register_locked_file(FILE *, struct __pthread *)`

**意图**: 在线程的 `stdio_locks` 链表中注册当前持有的 FILE。用于 `fork()` 时检测继承状态和线程退出时的清理。

#### `void __unlist_locked_file(FILE *)`

**意图**: 从线程的锁定文件链表中移除指定的 FILE。

#### `void __do_orphaned_stdio_locks(void)`

**意图**: 线程退出时清理该线程遗留的 FILE 锁。遍历 `stdio_locks` 链表，释放所有仍被持有的锁。

---

### 杂项函数

#### `void __getopt_msg(const char *, const char *, const char *, size_t)`

**意图**: `getopt` 内部使用的错误消息输出函数。

#### `FILE *__fopen_rb_ca(const char *, FILE *, unsigned char *, size_t)`

**意图**: "fopen read binary caller-allocated" — 打开文件供只读，使用调用者预分配的 FILE 结构体和缓冲区。

#### `int __fclose_ca(FILE *)`

**意图**: 对应 `__fopen_rb_ca` 的关闭操作。关闭 fd 但不释放调用者分配的 FILE/缓冲区。

---

## 内联宏定义

### `feof(f)` / `ferror(f)`

```c
#define feof(f) ((f)->flags & F_EOF)
#define ferror(f) ((f)->flags & F_ERR)
```

**意图**: 直接检查 FILE 的状态标志位。musl 的内联实现，避免函数调用开销。

### `getc_unlocked(f)`

```c
#define getc_unlocked(f) \
    ( ((f)->rpos != (f)->rend) ? *(f)->rpos++ : __uflow((f)) )
```

**意图**: 无锁版本的单字符读取。快速路径直接自缓冲区读取；慢速路径调用 `__uflow` 填充缓冲区。

### `putc_unlocked(c, f)`

```c
#define putc_unlocked(c, f) \
    ( (((unsigned char)(c)!=(f)->lbf && (f)->wpos!=(f)->wend)) \
    ? *(f)->wpos++ = (unsigned char)(c) \
    : __overflow((f),(unsigned char)(c)) )
```

**意图**: 无锁版本的单字符写入。快速路径直接写入缓冲区（条件是"写入非 `lbf` 字符（换行符）且缓冲区未满"）；慢速路径调用 `__overflow` 处理缓冲区溢出或行缓冲刷新。

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `__syscall`, `syscall` | `syscall.h`（musl 内部） | 底层 read/write/close 系统调用 |
| `FILE` 类型名 | `<stdio.h>` 标准头文件 | 外部定义（`typedef struct _IO_FILE FILE`） |
| `a_cas`, `a_inc` 等原子操作 | `atomic.h`（musl 内部） | 锁实现使用 |
| `FUTEX_WAIT/WAKE/PRIVATE` | `futex.h`（musl 内部） | futex 等待/唤醒 |
| `struct __locale_struct` | `libc.h`（musl 内部） | FILE 关联 locale |
| `struct __pthread` | `pthread_impl.h`（musl 内部） | 线程锁定文件链表 |

---

## 实现指南 (rusl/Rust)

- `struct _IO_FILE` → `#[repr(C)]` 的 Rust 结构体。函数指针成员用 `Option<unsafe extern "C" fn(...)>` 表示
- `FLOCK`/`FUNLOCK` → Rust 中使用 RAII 守卫模式：`FileLockGuard::new(file)` / `Drop`
- `lock` 字段 → `AtomicI32`（futex 兼容）
- 打开文件链表 → `Mutex<LinkedList>` 或 `RwLock<Vec<NonNull<_IO_FILE>>>`
- `read`/`write`/`seek`/`close` 虚函数 → Rust trait `FileOps` 的实现
- `flags` → `bitflags!` 宏实现 `F_*` 位标志
- `pipe_pid` → `Option<NonNull<c_void>>` 或 PID 类型封装
- `getc_unlocked` / `putc_unlocked` → `#[inline(always)]` 函数，快速路径在 LLVM 优化下生成高效代码
- 注意：musl 中 `FILE` 既是 `struct _IO_FILE` 的别名，需要确保 Rust FFI 类型映射正确