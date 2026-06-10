# stdio 内部实现模块 — 对外导出符号

本文件记录 `src/stdio/` 下所有文件的符号导出状态，分为内部实现和格式化 I/O 两部分。

---

## 对外导出（Public API）— 内部实现模块

| 符号 | 定义文件 | 说明 |
|------|----------|------|
| `fdopen` | `__fdopen.c` (weak_alias) | 通过文件描述符打开流 — POSIX 标准函数 |
| `__overflow` | `__overflow.c` | 输出缓冲区溢出处理 — 由 `putc_unlocked` 宏使用，protected 可见性 |
| `setbuf` | `setbuf.c` | 设置 FILE 流缓冲模式和缓冲区（BUFSIZ 大小） |
| `setvbuf` | `setvbuf.c` | 设置 FILE 流缓冲模式、缓冲区位置和大小 — 所有缓冲设置函数的底层实现 |
| `ungetc` | `ungetc.c` | 将单字节字符推回 FILE 流读缓冲区 |
| `ungetwc` | `ungetwc.c` | 将宽字符推回 FILE 流读缓冲区（支持多字节编码和 locale） |
| `stdin` | `stdin.c` | 标准输入 FILE 对象（文件描述符 0，全缓冲 BUFSIZ） |
| `stdout` | `stdout.c` | 标准输出 FILE 对象（文件描述符 1，行缓冲 BUFSIZ） |
| `stderr` | `stderr.c` | 标准错误输出 FILE 对象（文件描述符 2，无缓冲） |
| `setbuffer` | `setbuffer.c` | GNU 扩展：设置流缓冲模式和自定义大小缓冲区 |
| `setlinebuf` | `setlinebuf.c` | GNU 扩展：将流设为行缓冲模式 |

### 文件定位操作 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fseek` | `fseek.c` | ISO C | 设置文件位置指示符（`long` 偏移） |
| `fseeko` | `fseek.c` (weak_alias) | POSIX | 设置文件位置指示符（`off_t` 偏移，支持大文件） |
| `ftell` | `ftell.c` | ISO C | 获取当前文件位置（返回 `long`） |
| `ftello` | `ftell.c` (weak_alias) | POSIX | 获取当前文件位置（返回 `off_t`，支持大文件） |
| `fgetpos` | `fgetpos.c` | ISO C | 获取文件位置存入 `fpos_t` 不透明对象 |
| `fsetpos` | `fsetpos.c` | ISO C | 从 `fpos_t` 不透明对象恢复文件位置 |
| `rewind` | `rewind.c` | ISO C | 回绕到文件起始并清除错误标志 |

### 流状态查询 / 清除 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `feof` | `feof.c` | ISO C | 测试流文件结束指示符 |
| `ferror` | `ferror.c` | ISO C | 测试流错误指示符 |
| `clearerr` | `clearerr.c` | ISO C | 清除流的 EOF 和错误指示符 |
| `feof_unlocked` | `feof.c` (weak_alias) | POSIX | `feof` 的免锁版本 |
| `ferror_unlocked` | `ferror.c` (weak_alias) | POSIX | `ferror` 的免锁版本 |
| `clearerr_unlocked` | `clearerr.c` (weak_alias) | POSIX | `clearerr` 的免锁版本 |

### 流锁定操作 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `flockfile` | `flockfile.c` | POSIX | 获取 FILE 关联的内部递归锁（阻塞式） |
| `ftrylockfile` | `ftrylockfile.c` | POSIX | 尝试获取 FILE 关联的内部递归锁（非阻塞式） |
| `funlockfile` | `funlockfile.c` | POSIX | 释放 FILE 关联的内部递归锁 |

### 流底层文件描述符 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fileno` | `fileno.c` | POSIX | 获取与 FILE 流关联的底层文件描述符 |
| `fileno_unlocked` | `fileno.c` (weak_alias) | POSIX | `fileno` 的免锁版本 |

## 内部实现（不对外导出）— 内部实现模块

| 符号 | 定义文件 | 可见性 | 说明 |
|------|----------|--------|------|
| `__stdio_read` | `__stdio_read.c` | `hidden` | 默认 FILE 读操作函数指针实现 |
| `__stdio_write` | `__stdio_write.c` | `hidden` | 默认 FILE 写操作函数指针实现 |
| `__stdio_seek` | `__stdio_seek.c` | `hidden` | 默认 FILE 定位操作函数指针实现 |
| `__stdio_close` | `__stdio_close.c` | `hidden` | 默认 FILE 关闭操作函数指针实现 |
| `__stdout_write` | `__stdout_write.c` | `hidden` | stdout 专用写函数（检测终端行缓冲） |
| `__lockfile` | `__lockfile.c` | `hidden` | 获取 FILE 锁（支持 futex 等待） |
| `__unlockfile` | `__lockfile.c` | `hidden` | 释放 FILE 锁（支持 futex 唤醒） |
| `__fdopen` | `__fdopen.c` | `hidden` | 从 fd+mode 构造 FILE（`fdopen` 的主实现） |
| `__fmodeflags` | `__fmodeflags.c` | `hidden` | 将 mode 字符串转换为 open 标志 |
| `__fclose_ca` | `__fclose_ca.c` | `hidden` | 调用方分配 FILE 的关闭操作 |
| `__fopen_rb_ca` | `__fopen_rb_ca.c` | `hidden` | 调用方分配 FILE 的只读打开操作 |
| `__stdin_FILE` | `stdin.c` | `hidden` | stdin 的 FILE 结构体实例 |
| `__stdout_FILE` | `stdout.c` | `hidden` | stdout 的 FILE 结构体实例 |
| `__stderr_FILE` | `stderr.c` | `hidden` | stderr 的 FILE 结构体实例 |
| `__stdin_used` | `stdin.c` | `hidden` / weak_alias | stdin 哨兵变量（退出时刷新；弱别名兜底为 NULL） |
| `__stdout_used` | `stdout.c` | `hidden` / weak_alias | stdout 哨兵变量（退出时刷新；弱别名兜底为 NULL） |
| `__stderr_used` | `stderr.c` | `hidden` / weak_alias | stderr 哨兵变量（退出时刷新；弱别名兜底为 NULL） |
| `__stdio_exit` | `__stdio_exit.c` | `hidden` | 程序退出时的 stdio 清理（刷新所有缓冲、修正文件偏移） |
| `__stdio_exit_needed` | `__stdio_exit.c` | weak_alias (内部) | `__stdio_exit` 的弱别名，供 exit 路径符号引用链使用 |
| `__ofl_lock` | `ofl.c` | `hidden` | 获取全局打开文件链表锁，返回链表头指针 |
| `__ofl_unlock` | `ofl.c` | `hidden` | 释放全局打开文件链表锁 |
| `__ofl_add` | `ofl_add.c` | `hidden` | 将 FILE 插入全局打开文件链表头部 |
| `__stdio_ofl_lockptr` | `ofl.c` | 全局变量 | 指向 `ofl_lock` 的指针，供 fork 后锁重置使用 |
| `__fseeko` | `fseek.c` | `hidden` | 加锁版文件流定位（`fseeko` 的主实现） |
| `__fseeko_unlocked` | `fseek.c` | `hidden` | 不加锁版文件流定位引擎（缓冲区同步 + seek） |
| `__ftello` | `ftell.c` | `hidden` | 加锁版文件流位置查询（`ftello` 的主实现） |
| `__ftello_unlocked` | `ftell.c` | `hidden` | 不加锁版文件流位置查询（内核偏移 + 缓冲区补偿） |
| `__register_locked_file` | `ftrylockfile.c` | `hidden` | 将 FILE 注册到线程持有的 stdio 锁链表 |
| `__unlist_locked_file` | `ftrylockfile.c` | `hidden` | 将 FILE 从线程持有的 stdio 锁链表移除 |
| `__do_orphaned_stdio_locks` | `ftrylockfile.c` | `hidden` | 线程退出时清理所有孤儿 stdio 锁（设为 MAYBE_WAITERS） |

---

## 对外导出（Public API）— 格式化 I/O 模块

### printf 家族（格式化输出）

| 符号 | 定义文件 | 标准/扩展 | 说明 |
|------|----------|-----------|------|
| `printf` | `printf.c` | C89/C99 | 向 `stdout` 格式化输出 |
| `fprintf` | `fprintf.c` | C89/C99 | 向 `FILE` 流格式化输出 |
| `sprintf` | `sprintf.c` | C89/C99 | 向字符串缓冲区格式化输出（无边界检查） |
| `snprintf` | `snprintf.c` | C99 | 向字符串缓冲区格式化输出（有边界检查） |
| `asprintf` | `asprintf.c` | GNU/POSIX | 向动态分配缓冲区格式化输出 |
| `dprintf` | `dprintf.c` | POSIX.1-2008 | 向文件描述符格式化输出 |
| `vprintf` | `vprintf.c` | C89/C99 | `printf` 的 `va_list` 版本 |
| `vfprintf` | `vfprintf.c` | C89/C99 | `fprintf` 的 `va_list` 版本（核心引擎） |
| `vsprintf` | `vsprintf.c` | C89/C99 | `sprintf` 的 `va_list` 版本 |
| `vsnprintf` | `vsnprintf.c` | C99 | `snprintf` 的 `va_list` 版本 |
| `vasprintf` | `vasprintf.c` | GNU/POSIX | `asprintf` 的 `va_list` 版本 |
| `vdprintf` | `vdprintf.c` | POSIX.1-2008 | `dprintf` 的 `va_list` 版本 |

### scanf 家族（格式化输入）

| 符号 | 定义文件 | 标准/扩展 | 说明 |
|------|----------|-----------|------|
| `scanf` | `scanf.c` | C89/C99 | 从 `stdin` 格式化输入 |
| `fscanf` | `fscanf.c` | C89/C99 | 从 `FILE` 流格式化输入 |
| `sscanf` | `sscanf.c` | C89/C99 | 从字符串格式化输入 |
| `vscanf` | `vscanf.c` | C99 | `scanf` 的 `va_list` 版本 |
| `vfscanf` | `vfscanf.c` | C99 | `fscanf` 的 `va_list` 版本（核心引擎） |
| `vsscanf` | `vsscanf.c` | C99 | `sscanf` 的 `va_list` 版本 |

### 读写操作模块 — 对外导出

以下为 `fread.c`、`fwrite.c`、`fgetc.c`、`fputc.c`、`fgets.c`、`fputs.c`、`getc.c`、`putc.c`、`getchar.c`、`putchar.c`、`fgetln.c`、`getdelim.c`、`getline.c` 中对外可见的符号。

#### 标准 C 库接口 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fread` | `fread.c` | ISO C / POSIX | 从 FILE 流读取二进制块 |
| `fwrite` | `fwrite.c` | ISO C / POSIX | 向 FILE 流写入二进制块 |
| `fgetc` | `fgetc.c` | ISO C / POSIX | 从 FILE 流读取一个字符 |
| `fputc` | `fputc.c` | ISO C / POSIX | 向 FILE 流写入一个字符 |
| `fgets` | `fgets.c` | ISO C / POSIX | 从 FILE 流读取一行字符串 |
| `fputs` | `fputs.c` | ISO C / POSIX | 向 FILE 流写入字符串 |
| `getc` | `getc.c` | ISO C / POSIX | 从 FILE 流读取字符（宏的函数备选实现） |
| `putc` | `putc.c` | ISO C / POSIX | 向 FILE 流写入字符（宏的函数备选实现） |
| `getchar` | `getchar.c` | ISO C / POSIX | 从 stdin 读取一个字符 |
| `putchar` | `putchar.c` | ISO C / POSIX | 向 stdout 写入一个字符 |

#### POSIX 免锁扩展 (Public API)

| 符号 | 定义文件 | 说明 |
|------|----------|------|
| `fread_unlocked` | `fread.c` (weak_alias) | fread 的免锁版本（与 fread 共享实现） |
| `fwrite_unlocked` | `fwrite.c` (weak_alias) | fwrite 的免锁版本（与 fwrite 共享实现） |
| `fgets_unlocked` | `fgets.c` (weak_alias) | fgets 的免锁版本（与 fgets 共享实现） |
| `fputs_unlocked` | `fputs.c` (weak_alias) | fputs 的免锁版本（与 fputs 共享实现） |

#### POSIX.1-2008 动态行读取 (Public API)

| 符号 | 定义文件 | 说明 |
|------|----------|------|
| `getdelim` | `getdelim.c` | 以指定分隔符读取动态分配的一行 |
| `getline` | `getline.c` | 以 `\n` 为分隔符读取动态分配的一行 |

#### GNU 扩展 (Public — 需定义 `_GNU_SOURCE`)

| 符号 | 定义文件 | 说明 |
|------|----------|------|
| `fgetln` | `fgetln.c` | 零拷贝返回 FILE 缓冲区中下一行指针 |

#### 读写操作 — 内部符号

| 符号 | 定义文件 | 可见性 | 说明 |
|------|----------|--------|------|
| `__fwritex` | `fwrite.c` | `hidden` | 无锁底层缓冲写入引擎，被 fwrite/fputs/printf 系列调用 |
| `__getdelim` | `getdelim.c` | weak_alias (内部) | getdelim 的 `__` 前缀别名 |
| `_IO_getc` | `getc.c` | weak_alias (内部) | 传统 glibc `_IO_*` 兼容别名 |
| `_IO_putc` | `putc.c` | weak_alias (内部) | 传统 glibc `_IO_*` 兼容别名 |

---

## 宽字符 I/O 模块 — 对外导出

### 宽字符单字符 I/O (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fgetwc` | `fgetwc.c` | ISO C / POSIX | 从 FILE 流读取一个宽字符 |
| `fputwc` | `fputwc.c` | ISO C / POSIX | 向 FILE 流写入一个宽字符 |
| `getwc` | `getwc.c` | ISO C / POSIX | 从 FILE 流读取宽字符（宏的函数备选实现） |
| `putwc` | `putwc.c` | ISO C / POSIX | 向 FILE 流写入宽字符（宏的函数备选实现） |
| `getwchar` | `getwchar.c` | ISO C / POSIX | 从 stdin 读取一个宽字符 |
| `putwchar` | `putwchar.c` | ISO C / POSIX | 向 stdout 写入一个宽字符 |
| `fgetws` | `fgetws.c` | ISO C / POSIX | 从 FILE 流读取一行宽字符串 |
| `fputws` | `fputws.c` | ISO C / POSIX | 向 FILE 流写入宽字符串 |

#### 宽字符免锁扩展 (Public API)

| 符号 | 定义文件 | 说明 |
|------|----------|------|
| `fgetwc_unlocked` | `fgetwc.c` (weak_alias) | fgetwc 的免锁版本（指向 `__fgetwc_unlocked`） |
| `getwc_unlocked` | `fgetwc.c` (weak_alias) | getwc 的免锁版本（指向 `__fgetwc_unlocked`） |
| `fputwc_unlocked` | `fputwc.c` (weak_alias) | fputwc 的免锁版本（指向 `__fputwc_unlocked`） |
| `putwc_unlocked` | `fputwc.c` (weak_alias) | putwc 的免锁版本（指向 `__fputwc_unlocked`） |
| `fgetws_unlocked` | `fgetws.c` (weak_alias) | fgetws 的免锁版本（与 fgetws 共享实现） |
| `fputws_unlocked` | `fputws.c` (weak_alias) | fputws 的免锁版本（与 fputws 共享实现） |
| `getwchar_unlocked` | `getwchar.c` (weak_alias) | getwchar 的免锁版本（与 getwchar 共享实现） |
| `putwchar_unlocked` | `putwchar.c` (weak_alias) | putwchar 的免锁版本（与 putwchar 共享实现） |

### 宽字符格式化 I/O (Public API)

#### 宽字符 printf 家族（格式化输出）

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fwprintf` | `fwprintf.c` | ISO C / POSIX | 向 FILE 流格式化输出宽字符串 |
| `wprintf` | `wprintf.c` | ISO C / POSIX | 向 stdout 格式化输出宽字符串 |
| `swprintf` | `swprintf.c` | ISO C / POSIX | 向宽字符缓冲区格式化输出（有边界检查） |
| `vfwprintf` | `vfwprintf.c` | ISO C / POSIX | `fwprintf` 的 `va_list` 版本（核心引擎） |
| `vwprintf` | `vwprintf.c` | ISO C / POSIX | `wprintf` 的 `va_list` 版本 |
| `vswprintf` | `vswprintf.c` | ISO C / POSIX | `swprintf` 的 `va_list` 版本 |

#### 宽字符 scanf 家族（格式化输入）

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fwscanf` | `fwscanf.c` | ISO C / POSIX | 从 FILE 流格式化输入宽字符串 |
| `wscanf` | `wscanf.c` | ISO C / POSIX | 从 stdin 格式化输入宽字符串 |
| `swscanf` | `swscanf.c` | ISO C / POSIX | 从宽字符串格式化输入 |
| `vfwscanf` | `vfwscanf.c` | ISO C / POSIX | `fwscanf` 的 `va_list` 版本（核心引擎） |
| `vwscanf` | `vwscanf.c` | ISO C / POSIX | `wscanf` 的 `va_list` 版本 |
| `vswscanf` | `vswscanf.c` | ISO C / POSIX | `swscanf` 的 `va_list` 版本 |

### 其他宽字符函数 (Public API)

| 符号 | 定义文件 | 标准 | 说明 |
|------|----------|------|------|
| `fwide` | `fwide.c` | ISO C / POSIX | 设置/查询 FILE 流的宽窄方向 |
| `open_wmemstream` | `open_wmemstream.c` | POSIX.1-2008 | 打开动态宽字符内存流 |

### 宽字符模块 — 内部实现（不对外导出）

| 符号 | 定义文件 | 可见性 | 说明 |
|------|----------|--------|------|
| `__fgetwc_unlocked` | `fgetwc.c` | `hidden` | 无锁宽字符读取（`fgetwc_unlocked`/`getwc_unlocked` 的主实现） |
| `__fgetwc_unlocked_internal` | `fgetwc.c` | `static` | 宽字符读取核心引擎（多字节转换逻辑） |
| `__fputwc_unlocked` | `fputwc.c` | `hidden` | 无锁宽字符写入（`fputwc_unlocked`/`putwc_unlocked` 的主实现） |
| `wprintf_core` | `vfwprintf.c` | `static` | 宽字符 printf 格式化核心引擎 |
| `wstring_read` | `vswscanf.c` | `static` | vswscanf 自定义读取回调（宽字符串源） |
| `sw_write` | `vswprintf.c` | `static` | vswprintf 自定义写入回调（宽字符目标） |
| `wms_write` | `open_wmemstream.c` | `static` | 宽字符内存流写入回调 |
| `wms_seek` | `open_wmemstream.c` | `static` | 宽字符内存流 seek 回调 |
| `wms_close` | `open_wmemstream.c` | `static` | 宽字符内存流关闭回调 |
| `struct cookie` (wms) | `open_wmemstream.c` | 文件作用域 | 宽字符内存流状态结构 |
| `struct wms_FILE` | `open_wmemstream.c` | 文件作用域 | 宽字符内存流 FILE 包装结构 |

---

### musl 内部兼容弱别名（不直接导出给用户）

| 弱别名 | 主符号 | 定义文件 | 说明 |
|--------|--------|----------|------|
| `__isoc99_scanf` | `scanf` | `scanf.c` | C99 标准兼容别名 |
| `__isoc99_fscanf` | `fscanf` | `fscanf.c` | C99 标准兼容别名 |
| `__isoc99_sscanf` | `sscanf` | `sscanf.c` | C99 标准兼容别名 |
| `__isoc99_vfscanf` | `vfscanf` | `vfscanf.c` | C99 标准兼容别名 |
| `__isoc99_vscanf` | `vscanf` | `vscanf.c` | C99 标准兼容别名 |
| `__isoc99_vsscanf` | `vsscanf` | `vsscanf.c` | C99 标准兼容别名 |
| `_IO_feof_unlocked` | `feof` | `feof.c` | glibc `_IO_*` 兼容别名 |
| `_IO_ferror_unlocked` | `ferror` | `ferror.c` | glibc `_IO_*` 兼容别名 |
| `_IO_getc_unlocked` | `getc_unlocked` | `getc_unlocked.c` | glibc `_IO_*` 兼容别名 |
| `_IO_putc_unlocked` | `putc_unlocked` | `putc_unlocked.c` | glibc `_IO_*` 兼容别名 |
| `__isoc99_fwscanf` | `fwscanf` | `fwscanf.c` | C99 宽字符 scanf 兼容别名 |
| `__isoc99_wscanf` | `wscanf` | `wscanf.c` | C99 宽字符 scanf 兼容别名 |
| `__isoc99_swscanf` | `swscanf` | `swscanf.c` | C99 宽字符 scanf 兼容别名 |
| `__isoc99_vfwscanf` | `vfwscanf` | `vfwscanf.c` | C99 宽字符 vfscanf 兼容别名 |
| `__isoc99_vwscanf` | `vwscanf` | `vwscanf.c` | C99 宽字符 vscanf 兼容别名 |
| `__isoc99_vswscanf` | `vswscanf` | `vswscanf.c` | C99 宽字符 vsscanf 兼容别名 |

### 格式化 I/O 模块 — 内部实现（不对外导出）

| 符号 | 定义文件 | 可见性 | 说明 |
|------|----------|--------|------|
| `printf_core` | `vfprintf.c` | `static` | printf 格式化核心引擎 |
| `pop_arg` | `vfprintf.c` | `static` | 从 va_list 提取参数 |
| `out` | `vfprintf.c` | `static` | 向 FILE 输出字节 |
| `pad` | `vfprintf.c` | `static` | 输出填充字符 |
| `fmt_x` / `fmt_o` / `fmt_u` | `vfprintf.c` | `static` | 整数格式化 |
| `fmt_fp` | `vfprintf.c` | `static` | 浮点数格式化 |
| `getint` | `vfprintf.c` | `static` | 解析格式串中整数 |
| `states` 数组 | `vfprintf.c` | `static` | 格式说明符状态机表 |
| `store_int` | `vfscanf.c` | `static` | 按长度修饰符存储整数 |
| `arg_n` | `vfscanf.c` | `static` | 按位置参数索引提取参数 |
| `sn_write` | `vsnprintf.c` | `static` | snprintf 输出回调 |
| `string_read` | `vsscanf.c` | `static` | sscanf 输入回调 |
| `struct cookie` | `vsnprintf.c` | 文件作用域 | snprintf 缓冲区管理结构 |
