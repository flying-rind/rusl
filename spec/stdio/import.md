# stdio 内部实现模块 — 外部依赖导入

本文件记录 `src/stdio/` 下所有函数使用到的来自外部模块的 C 接口，分为内部实现模块和格式化 I/O 模块两部分。

---

## 系统调用接口（来自内核）

| 接口 | 来源 | 说明 |
|------|------|------|
| `SYS_readv` / `syscall(SYS_readv, ...)` | 内核 (syscall) | 分散读取系统调用 |
| `SYS_read` / `syscall(SYS_read, ...)` | 内核 (syscall) | 普通读取系统调用 |
| `SYS_writev` / `syscall(SYS_writev, ...)` | 内核 (syscall) | 聚集写入系统调用 |
| `SYS_close` / `syscall(SYS_close, ...)` | 内核 (syscall) | 关闭文件描述符 |
| `SYS_ioctl` / `__syscall(SYS_ioctl, ...)` | 内核 (syscall) | 设备控制操作 |
| `SYS_fcntl` / `__syscall(SYS_fcntl, ...)` | 内核 (syscall) | 文件描述符控制 |
| `sys_open(...)` | 内核 (syscall) | 打开文件（通过 `__sys_open` 宏） |
| `__lseek(fd, off, whence)` | `unistd.h` (libc) / 内核 | 文件定位系统调用 |

## 结构体与类型

| 类型 | 来源 | 说明 |
|------|------|------|
| `struct iovec` | `<sys/uio.h>` | 散布/聚集 I/O 向量 |
| `struct winsize` | `<sys/ioctl.h>` | 终端窗口大小 |
| `FILE` / `struct _IO_FILE` | `stdio_impl.h` (内部) | musl 内部文件流结构体 |
| `struct __libc` | `libc.h` (内部) | musl 全局运行时状态 |

## 宏与常量

| 宏/常量 | 来源 | 说明 |
|----------|------|------|
| `F_ERR`, `F_EOF`, `F_NOWR`, `F_NORD`, `F_APP`, `F_SVB`, `F_PERM` | `stdio_impl.h` (内部) | 文件流标志位 |
| `UNGET`, `BUFSIZ` | `stdio_impl.h` (内部) | 回退缓冲区大小 / 默认缓冲大小 |
| `MAYBE_WAITERS` | `stdio_impl.h` (内部) | 锁等待者标志位 |
| `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_EXCL`, `O_CLOEXEC`, `O_CREAT`, `O_TRUNC`, `O_APPEND` | `<fcntl.h>` | open 系统调用标志 |
| `FD_CLOEXEC`, `F_SETFD`, `F_SETFL`, `F_GETFL` | `<fcntl.h>` | fcntl 命令/标志 |
| `EINVAL`, `EOVERFLOW`, `EBADF`, `errno` | `<errno.h>` | 错误码与错误号宏 |
| `LONG_MAX` | `<limits.h>` | long 类型最大值（ftrylockfile 递归计数溢出检查 / ftell 溢出检查） |
| `EOF` | `<stdio.h>` | 文件结束标志 |
| `TIOCGWINSZ` | `<sys/ioctl.h>` (平台相关) | 获取窗口大小的 ioctl 请求码 |

## libc 函数

| 接口 | 来源 | 说明 |
|------|------|------|
| `malloc(size)` | `<stdlib.h>` | 动态内存分配 |
| `memset(ptr, val, n)` | `<string.h>` | 内存填充 |
| `strchr(s, c)` | `<string.h>` | 字符查找 |
| `memcpy(dst, src, n)` | `<string.h>` | 内存拷贝（ungetwc 多字节序列推回） |
| `wcrtomb(s, wc, ps)` | `<wchar.h>` | 宽字符到多字节转换（ungetwc 核心依赖） |
| `fwide(f, mode)` | `<wchar.h>` | 设置/查询流宽窄模式（ungetwc 流方向设置） |
| `isascii(c)` | `<ctype.h>` | ASCII 字符检测（ungetwc 快速路径判断） |
| `__lock(ptr)` / `__unlock(ptr)` | `lock.h` (内部) | 自旋锁操作（ofl.c 链表锁底层实现） |
| `__pthread_self()` | `pthread_impl.h` (内部) | 获取当前线程控制块指针 |

## 原子操作与同步原语

| 接口 | 来源 | 说明 |
|------|------|------|
| `a_cas(p, t, s)` | `atomic.h` (内部) | 原子比较交换 (CAS) |
| `a_swap(p, v)` | `atomic.h` (内部) | 原子交换 |
| `a_store(p, v)` | `atomic.h` (内部) | 原子存储（用于孤儿锁清理） |
| `__futexwait(addr, val, priv)` | `futex.h` (内部) | futex 等待 |
| `__wake(addr, cnt, priv)` | `futex.h` (内部) | futex 唤醒 |

## 跨模块内部依赖

| 接口 | 定义位置 | 说明 |
|------|----------|------|
| `__stdio_read` | `src/stdio/__stdio_read.c` | 默认读操作 (被 __fdopen, __fopen_rb_ca 引用) |
| `__stdio_write` | `src/stdio/__stdio_write.c` | 默认写操作 (被 __fdopen, __stdout_write 引用) |
| `__stdio_seek` | `src/stdio/__stdio_seek.c` | 默认定位操作 (被 __fdopen, __fopen_rb_ca 引用) |
| `__stdio_close` | `src/stdio/__stdio_close.c` | 默认关闭操作 (被 __fdopen, __fopen_rb_ca 引用) |
| `__towrite` | `src/stdio/__towrite.c` | 将流切换为写模式 (被 __overflow 调用) |
| `__toread` | `src/stdio/__toread.c` | 将流切换为读模式 (被 ungetc / ungetwc / fgetc 调用) |
| `__ofl_add` | `src/stdio/ofl_add.c` | 将 FILE 加入全局打开文件链表 (被 __fdopen 调用) |
| `__ofl_lock` / `__ofl_unlock` | `src/stdio/ofl.c` | 全局文件链表锁 |
| `__aio_close` | `aio_impl.h` (弱别名，默认 dummy) | AIO 关闭回调 (被 __stdio_close 调用) |
| `libc` (全局变量) | `libc.h` | 全局运行时状态 (`.threaded` 字段) |

---

## 格式化 I/O 模块 — 外部依赖

### 来自 `<stdio.h>` / `stdio_impl.h`（标准 I/O 库自身）

| 接口 | 来源 | 说明 |
|------|------|------|
| `FILE` / `struct _IO_FILE` | `stdio_impl.h` (内部) | I/O 流结构体，格式化 I/O 核心抽象 |
| `stdout` | `src/stdio/__stdout_used.c` | 标准输出流 (printf / vprintf) |
| `stdin` | `src/stdio/__stdin_used.c` | 标准输入流 (scanf / vscanf) |
| `ferror()` | `src/stdio/ferror.c` | 检查流的错误状态 |
| `__fwritex()` | `src/stdio/__fwritex.c` | 无锁写入 FILE 缓冲区 (out 函数调用) |
| `__towrite()` | `src/stdio/__towrite.c` | 准备流进入写模式 (vfprintf) |
| `__toread()` | `src/stdio/__toread.c` | 准备流进入读模式 (vfscanf) |
| `__stdio_write()` | `src/stdio/__stdio_write.c` | 直接文件描述符写入 (vdprintf) |
| `FLOCK()` / `FUNLOCK()` | `stdio_impl.h` (内部) | 流锁定/解锁宏 |
| `F_ERR` / `EOF` | `stdio_impl.h` / `<stdio.h>` | 流错误标志位 / 文件结束常量 |

### 来自 `shgetc.h`（格式化输入扫描辅助）

| 接口 | 来源 | 说明 |
|------|------|------|
| `__shlim()` | `src/stdio/__shlim.c` | 设置扫描宽度限制 |
| `__shgetc()` | `src/stdio/__shgetc.c` | 扫描缓冲下一字符 |
| `shlim()` | `shgetc.h` (宏) | 调用 `__shlim` |
| `shgetc()` | `shgetc.h` (宏) | 缓冲/调用 `__shgetc` |
| `shunget()` | `shgetc.h` (宏) | 回退一个字符 |
| `shcnt()` | `shgetc.h` (宏) | 获取已扫描字符数 |

### 来自 `intscan.h` / `floatscan.h`（数字扫描引擎）

| 接口 | 来源 | 说明 |
|------|------|------|
| `__intscan()` | `src/internal/intscan.c` | 整数扫描核心 (vfscanf %d/%x/%o 等) |
| `__floatscan()` | `src/internal/floatscan.c` | 浮点扫描核心 (vfscanf %f/%e/%g 等) |

### 来自 `<string.h>`（字符串操作）

| 接口 | 来源 | 说明 |
|------|------|------|
| `strerror()` | `src/string/strerror.c` | `%m` 格式的错误信息字符串 |
| `strnlen()` | `src/string/strnlen.c` | `%s` 格式的安全长度计算 |
| `memcpy()` | `src/string/memcpy.c` | 内存拷贝 (vsnprintf / vsscanf 回调) |
| `memchr()` | `src/string/memchr.c` | 查找 '\0' 终止符 (vsscanf 回调) |
| `memset()` | `src/string/memset.c` | 内存填充 (vfscanf 扫描集初始化) |

### 来自 `<wchar.h>`（宽字符支持）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `wctomb()` | `src/multibyte/wctomb.c` | 宽字符到多字节转换 | `fputwc.c`, `vfprintf.c`, `vfwscanf.c` |
| `mbtowc()` | `src/multibyte/mbtowc.c` | 无状态多字节到宽字符转换 | `fgetwc.c`, `vfwprintf.c`, `vswprintf.c` |
| `mbrtowc()` | `src/multibyte/mbrtowc.c` | 有状态多字节到宽字符转换 | `fgetwc.c`, `vfscanf.c` |
| `mbsnrtowcs()` | `src/multibyte/mbsnrtowcs.c` | 有状态多字节到宽字符串转换（限制长度） | `open_wmemstream.c` |
| `wcsrtombs()` | `src/multibyte/wcsrtombs.c` | 宽字符串到多字节字符串转换（可重启） | `fputws.c`, `vswscanf.c` |
| `wcsnlen()` | `src/string/wcsnlen.c` | 宽字符串安全长度计算 | `vfwprintf.c` |
| `btowc()` | `src/multibyte/btowc.c` | 单字节到宽字符转换 | `vfwprintf.c` |
| `mbsinit()` | `src/multibyte/mbsinit.c` | 检查多字节转换状态 | `vfscanf.c` |
| `fwide()` | `src/stdio/fwide.c` | 设置/查询流方向 | 几乎所有宽字符 I/O 函数 |

### 来自 `<wctype.h>`（宽字符分类）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `iswdigit()` | `src/ctype/iswdigit.c` | 宽字符数字判断 | `vfwprintf.c`, `vfwscanf.c` |
| `iswspace()` | `src/ctype/iswspace.c` | 宽字符空白判断 | `vfwscanf.c` |

### 来自 locale_impl.h（locale 管理）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `CURRENT_LOCALE` | `locale_impl.h` (内部) | 每线程当前 locale 指针 | `fgetwc.c`, `fputwc.c`, `fputws.c` |
| `C_LOCALE` | `locale_impl.h` (内部) | C locale 常量 | `fwide.c` |
| `UTF8_LOCALE` | `locale_impl.h` (内部) | UTF-8 locale 常量 | `fwide.c` |

### 来自 `<limits.h>`（宽字符相关）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `MB_LEN_MAX` | `<limits.h>` | 多字节字符最大字节数 | `fputwc.c`, `vfwprintf.c` |
| `SSIZE_MAX` | `<limits.h>` | `ssize_t` 最大值（溢出保护） | `open_wmemstream.c` |

### 来自 `<errno.h>`（宽字符错误码）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `EILSEQ` | `<errno.h>` | 非法字节序列错误码 | `fgetwc.c` |

### 来自 `<stdlib.h>`（动态内存）

| 接口 | 来源 | 说明 |
|------|------|------|
| `malloc()` | `src/malloc/malloc.c` | 动态内存分配 (asprintf / `%m` 模式 / vfscanf) |
| `realloc()` | `src/malloc/realloc.c` | 动态内存重新分配 (`%m` 模式缓冲区扩展) |
| `free()` | `src/malloc/free.c` | 动态内存释放 (`%m` 模式失败清理) |

### 来自 `<ctype.h>`（字符分类）

| 接口 | 来源 | 说明 |
|------|------|------|
| `isspace()` | `src/ctype/isspace.c` | 判断空白字符 (格式化字符串解析) |
| `isdigit()` | `src/ctype/isdigit.c` | 判断十进制数字 (宽度/精度解析) |

### 来自 `<math.h>` / `<float.h>`（浮点支持）

| 接口 | 来源 | 说明 |
|------|------|------|
| `frexpl()` | `src/math/frexpl.c` | 提取 long double 尾数和指数 (`%a` 格式) |
| `signbit()` | `<math.h>` | 浮点数符号位测试 |
| `scalbn()` | `src/math/scalbn.c` | 乘以 2^N (浮点舍入调整) |
| `isfinite()` | `<math.h>` | 有限值测试 (NaN/Inf 检测) |
| `LDBL_MANT_DIG` / `LDBL_MAX_EXP` / `LDBL_EPSILON` | `<float.h>` | long double 精度/范围常量 |
| `DBL_MANT_DIG` / `DBL_MAX_EXP` | `<float.h>` | double 精度/范围常量 |

### 来自 `<limits.h>`（整数类型边界）

| 接口 | 来源 | 说明 |
|------|------|------|
| `INT_MAX` | `<limits.h>` | 溢出检测 / vsprintf 无界参数 |
| `ULONG_MAX` | `<limits.h>` | fmt_u 优化分界点 |
| `INTMAX_MAX` | `<limits.h>` | %d/%i 负号处理 |
| `NL_ARGMAX` | `<limits.h>` | 位置参数 `$` 最大索引 (=9) |
| `MB_LEN_MAX` | `<limits.h>` | 多字节字符最大字节数（ungetwc 转换缓冲区大小） |

### 来自 `<errno.h>`（错误码）

| 接口 | 来源 | 说明 |
|------|------|------|
| `errno` | `src/errno/__errno_location.c` | 错误码全局变量 |
| `EINVAL` | `<errno.h>` | 非法参数错误 (格式字符串错误) |
| `EOVERFLOW` | `<errno.h>` | 数值溢出错误 (格式化输出溢出) |

### 来自 `<stdarg.h>`（可变参数标准库）

| 接口 | 来源 | 说明 |
|------|------|------|
| `va_list` / `va_start()` / `va_end()` / `va_copy()` / `va_arg()` | `<stdarg.h>` (编译器内置) | 可变参数操作 |

### 来自 `<stdint.h>` / `<stddef.h>`（标准类型）

| 接口 | 来源 | 说明 |
|------|------|------|
| `uintmax_t` / `intmax_t` / `uintptr_t` / `ptrdiff_t` | `<stdint.h>` / `<stddef.h>` | 格式化 I/O 所需的通用整数类型 |
| `size_t` | `<stddef.h>` | 无符号大小类型 |
| `ssize_t` | `<sys/types.h>` | 有符号大小类型（POSIX），用于 getdelim/getline 返回读取字符数 |
| `mbstate_t` | `<wchar.h>` | 多字节转换状态 |
| `SIZE_MAX` | `<stdint.h>` / `<inttypes.h>` | 最大 size_t 值，getdelim.c 用于防止溢出 |

---

## 读写操作模块 — 特定依赖

以下为 `fread.c`、`fwrite.c`、`fgetc.c`、`fputc.c`、`fgets.c`、`fputs.c`、`getc.c`、`putc.c`、`getchar.c`、`putchar.c`、`fgetln.c`、`getdelim.c`、`getline.c` 及其辅助头文件 `getc.h`、`putc.h` 所需的外部模块接口。

### 来自 `<string.h>`（字符串操作）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `memcpy` | `<string.h>` | FILE 缓冲区与用户缓冲区之间的批量数据搬运 | `fread.c`, `fwrite.c`, `fgets.c`, `getdelim.c` |
| `memchr` | `<string.h>` | 在 FILE 读缓冲区中搜索换行符或分隔符 | `fgets.c`, `fgetln.c`, `getdelim.c` |
| `strlen` | `<string.h>` | 计算 C 字符串长度（fputs 确定写入量） | `fputs.c` |

### 来自 `<stdlib.h>`（动态内存）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `realloc` | `<stdlib.h>` | 动态扩展行输出缓冲区（getdelim 核心依赖） | `getdelim.c` |

### 来自 `<stdio.h>`（标准 I/O）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `ungetc` | `<stdio.h>` | 将字符推回 FILE 流的读缓冲区 | `fgetln.c` |
| `stdin` | `<stdio.h>` (全局变量) | 标准输入 FILE 指针 | `getchar.c` |
| `stdout` | `<stdio.h>` (全局变量) | 标准输出 FILE 指针 | `putchar.c` |

### 来自 `<errno.h>`（错误码）

| 接口 | 来源 | 说明 | 使用者 |
|------|------|------|--------|
| `EINVAL` | `<errno.h>` | 无效参数错误码（getdelim 参数校验） | `getdelim.c` |
| `ENOMEM` | `<errno.h>` | 内存不足错误码（getdelim realloc 失败） | `getdelim.c` |

### FILE 结构体字段直接访问

`getc.h` 和 `putc.h` 中的 `do_getc`/`do_putc` 直接访问 `_IO_FILE` 的以下字段（定义于 `stdio_impl.h`）：

| 字段 | 类型 | 用途 |
|------|------|------|
| `f->lock` | `volatile int` | FILE 锁。`< 0` 免锁；`>= 0` 时低 30 位为持有者 tid，位 30 为 `MAYBE_WAITERS` |
| `f->rpos` | `unsigned char *` | 读缓冲区当前位置指针 |
| `f->rend` | `unsigned char *` | 读缓冲区末尾指针 |
| `f->wpos` | `unsigned char *` | 写缓冲区当前位置指针 |
| `f->wend` | `unsigned char *` | 写缓冲区末尾指针 |
| `f->lbf` | `int` | 行缓冲分隔符（正值 `'\n'` 行缓冲；`EOF` 全缓冲） |
| `f->mode` | `int` | 读写模式标志（最低位：0 未设置，1 写模式） |
| `f->flags` | `unsigned` | FILE 状态标志（F_EOF/F_ERR/F_NORD/F_NOWR 等） |
| `f->read` | `size_t (*)(FILE *, unsigned char *, size_t)` | 底层读取函数指针 |
| `f->write` | `size_t (*)(FILE *, const unsigned char *, size_t)` | 底层写入函数指针 |
| `f->getln_buf` | `char *` | GNU fgetln 使用的动态行缓冲区 |
