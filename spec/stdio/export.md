# stdio 模块 — 对外导出 API 汇总

本文件记录 `src/stdio/` 下所有在 `<stdio.h>` 中声明的、用户程序可直接调用的公开接口。内部实现符号（`__` 前缀、`hidden` 可见性、弱别名等）不在此列，详见各具体 spec 文件。

---

## 宏常量

| 符号 | 标准 | 值 | 说明 |
|------|------|-----|------|
| `NULL` | ISO C | `((void*)0)` | 空指针常量 |
| `EOF` | ISO C | `(-1)` | 文件结束/错误返回值 |
| `SEEK_SET` | ISO C | `0` | 文件定位 — 从文件头开始 |
| `SEEK_CUR` | ISO C | `1` | 文件定位 — 从当前位置开始 |
| `SEEK_END` | ISO C | `2` | 文件定位 — 从文件尾开始 |
| `_IOFBF` | ISO C | `0` | 全缓冲模式 |
| `_IOLBF` | ISO C | `1` | 行缓冲模式 |
| `_IONBF` | ISO C | `2` | 无缓冲模式 |
| `BUFSIZ` | ISO C | `1024` | 默认缓冲区大小 |
| `FILENAME_MAX` | ISO C | `4096` | 文件名最大长度 |
| `FOPEN_MAX` | ISO C | `1000` | 同时打开的最大文件数 |
| `TMP_MAX` | ISO C | `10000` | tmpnam 可生成的最大唯一文件名数 |
| `L_tmpnam` | ISO C | `20` | tmpnam 缓冲区所需最小长度 |
| `L_ctermid` | POSIX | `20` | ctermid 缓冲区所需最小长度 |
| `P_tmpdir` | XOPEN/GNU/BSD | `"/tmp"` | 默认临时文件目录路径 |
| `L_cuserid` | GNU/BSD | `20` | cuserid 缓冲区所需最小长度 |
| `RENAME_NOREPLACE` | GNU | `(1 << 0)` | renameat2 标志 — 目标存在则失败 |
| `RENAME_EXCHANGE` | GNU | `(1 << 1)` | renameat2 标志 — 原子交换 |
| `RENAME_WHITEOUT` | GNU | `(1 << 2)` | renameat2 标志 — 白化源文件 |

### _LARGEFILE64_SOURCE 兼容宏（`#define` 别名）

以下宏在定义 `_LARGEFILE64_SOURCE` 时生效，将 `xxx64` 符号映射到对应的标准函数/类型：

| 宏 | 映射目标 | 说明 |
|-----|---------|------|
| `tmpfile64` | `tmpfile` | 64 位临时文件（透明别名） |
| `fopen64` | `fopen` | 64 位文件打开（透明别名） |
| `freopen64` | `freopen` | 64 位文件重打开（透明别名） |
| `fseeko64` | `fseeko` | 64 位文件定位（透明别名） |
| `ftello64` | `ftello` | 64 位文件位置查询（透明别名） |
| `fgetpos64` | `fgetpos` | 64 位文件位置获取（透明别名） |
| `fsetpos64` | `fsetpos` | 64 位文件位置设置（透明别名） |
| `fpos64_t` | `fpos_t` | 64 位文件位置类型（透明别名） |
| `off64_t` | `off_t` | 64 位文件偏移类型（透明别名） |

---

## 类型定义

| 符号 | 标准 | 说明 |
|------|------|------|
| `fpos_t` | ISO C | 文件位置不透明类型，定义为 `union _G_fpos64_t { char __opaque[16]; long long __lldata; double __align; }` |
| `cookie_read_function_t` | GNU | Cookie 流读取回调：`ssize_t (*)(void *, char *, size_t)` |
| `cookie_write_function_t` | GNU | Cookie 流写入回调：`ssize_t (*)(void *, const char *, size_t)` |
| `cookie_seek_function_t` | GNU | Cookie 流定位回调：`int (*)(void *, off_t *, int)` |
| `cookie_close_function_t` | GNU | Cookie 流关闭回调：`int (*)(void *)` |
| `cookie_io_functions_t` | GNU | Cookie 流回调集结构体，含 `read` / `write` / `seek` / `close` 四个函数指针 |

---

## 标准流对象

| 符号 | 类型 | 标准 | 说明 |
|------|------|------|------|
| `stdin` | `FILE *const` | ISO C | 标准输入流（文件描述符 0） |
| `stdout` | `FILE *const` | ISO C | 标准输出流（文件描述符 1） |
| `stderr` | `FILE *const` | ISO C | 标准错误输出流（文件描述符 2） |

---

## 1. 文件操作

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `fopen` | `FILE *fopen(const char *path, const char *mode)` | 按模式打开文件，返回 FILE 流指针 |
| `freopen` | `FILE *freopen(const char *path, const char *mode, FILE *f)` | 以新模式重新打开流 |
| `fclose` | `int fclose(FILE *f)` | 关闭流、刷新缓冲并释放 FILE 对象 |
| `remove` | `int remove(const char *path)` | 删除文件 |
| `rename` | `int rename(const char *old, const char *new)` | 重命名文件 |
| `tmpfile` | `FILE *tmpfile(void)` | 创建临时文件（关闭时自动删除） |
| `tmpnam` | `char *tmpnam(char *s)` | 生成唯一临时文件名 |

### POSIX 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `renameat` | `int renameat(int oldfd, const char *old, int newfd, const char *new)` | 相对目录文件描述符的重命名 |
| `popen` | `FILE *popen(const char *cmd, const char *mode)` | 通过管道执行 shell 命令并打开流 |
| `pclose` | `int pclose(FILE *f)` | 关闭 popen 打开的流并等待命令结束 |

### GNU 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `renameat2` | `int renameat2(int oldfd, const char *old, int newfd, const char *new, unsigned flags)` | 带标志的 renameat（支持 NOREPLACE / EXCHANGE / WHITEOUT） |

---

## 2. 流缓冲

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `setbuf` | `void setbuf(FILE *f, char *buf)` | 设置流缓冲（等价 `setvbuf(f, buf, buf ? _IOFBF : _IONBF, BUFSIZ)`） |
| `setvbuf` | `int setvbuf(FILE *f, char *buf, int mode, size_t size)` | 设置流缓冲模式、缓冲区及大小 |

### GNU/BSD 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `setbuffer` | `void setbuffer(FILE *f, char *buf, size_t size)` | 设置全缓冲（mode 固定为 `_IOFBF`） |
| `setlinebuf` | `void setlinebuf(FILE *f)` | 设置行缓冲（等价 `setvbuf(f, NULL, _IOLBF, 0)`） |

---

## 3. 格式化输出（printf 家族）

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `printf` | `int printf(const char *fmt, ...)` | 向 stdout 格式化输出 |
| `fprintf` | `int fprintf(FILE *f, const char *fmt, ...)` | 向 FILE 流格式化输出 |
| `sprintf` | `int sprintf(char *s, const char *fmt, ...)` | 向字符串缓冲区格式化输出（无边界检查） |
| `snprintf` | `int snprintf(char *s, size_t n, const char *fmt, ...)` | 向字符串缓冲区格式化输出（有边界检查） |
| `vprintf` | `int vprintf(const char *fmt, va_list ap)` | `printf` 的 `va_list` 版本 |
| `vfprintf` | `int vfprintf(FILE *f, const char *fmt, va_list ap)` | `fprintf` 的 `va_list` 版本（核心引擎） |
| `vsprintf` | `int vsprintf(char *s, const char *fmt, va_list ap)` | `sprintf` 的 `va_list` 版本 |
| `vsnprintf` | `int vsnprintf(char *s, size_t n, const char *fmt, va_list ap)` | `snprintf` 的 `va_list` 版本 |

### POSIX 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `dprintf` | `int dprintf(int fd, const char *fmt, ...)` | 向文件描述符格式化输出 |
| `vdprintf` | `int vdprintf(int fd, const char *fmt, va_list ap)` | `dprintf` 的 `va_list` 版本 |

### GNU/BSD 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `asprintf` | `int asprintf(char **sp, const char *fmt, ...)` | 动态分配缓冲区并格式化输出 |
| `vasprintf` | `int vasprintf(char **sp, const char *fmt, va_list ap)` | `asprintf` 的 `va_list` 版本 |

---

## 4. 格式化输入（scanf 家族）

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `scanf` | `int scanf(const char *fmt, ...)` | 从 stdin 格式化输入 |
| `fscanf` | `int fscanf(FILE *f, const char *fmt, ...)` | 从 FILE 流格式化输入 |
| `sscanf` | `int sscanf(const char *s, const char *fmt, ...)` | 从字符串格式化输入 |
| `vscanf` | `int vscanf(const char *fmt, va_list ap)` | `scanf` 的 `va_list` 版本 |
| `vfscanf` | `int vfscanf(FILE *f, const char *fmt, va_list ap)` | `fscanf` 的 `va_list` 版本（核心引擎） |
| `vsscanf` | `int vsscanf(const char *s, const char *fmt, va_list ap)` | `sscanf` 的 `va_list` 版本 |

---

## 5. 字符 I/O

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `fgetc` | `int fgetc(FILE *f)` | 从流读取一个 `unsigned char` 转换为 `int` |
| `fputc` | `int fputc(int c, FILE *f)` | 向流写入一个字符（`int` 低 8 位） |
| `getc` | `int getc(FILE *f)` | 从流读取一个字符（宏的函数备选实现） |
| `putc` | `int putc(int c, FILE *f)` | 向流写入一个字符（宏的函数备选实现） |
| `getchar` | `int getchar(void)` | 从 stdin 读取一个字符（等价 `getc(stdin)`） |
| `putchar` | `int putchar(int c)` | 向 stdout 写入一个字符（等价 `putc(c, stdout)`） |
| `ungetc` | `int ungetc(int c, FILE *f)` | 将字符推回流读缓冲区 |

---

## 6. 字符串 I/O

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `fgets` | `char *fgets(char *s, int n, FILE *f)` | 从流读取一行至多 n-1 字符，保留换行符 |
| `fputs` | `int fputs(const char *s, FILE *f)` | 向流写入字符串（不含尾部 `\0`） |
| `puts` | `int puts(const char *s)` | 向 stdout 写入字符串并追加换行符 |
| `gets` | `char *gets(char *s)` | (C89，C11 中已移除) 从 stdin 读取一行（无边界检查，不安全） |

---

## 7. 块 I/O

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `fread` | `size_t fread(void *ptr, size_t size, size_t nmemb, FILE *f)` | 从流中读取最多 `nmemb` 个 `size` 大小的元素 |
| `fwrite` | `size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *f)` | 向流中写入最多 `nmemb` 个 `size` 大小的元素 |

---

## 8. 文件定位

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `fseek` | `int fseek(FILE *f, long offset, int whence)` | 设置文件位置指示符（`long` 偏移） |
| `ftell` | `long ftell(FILE *f)` | 获取当前文件位置（返回 `long`） |
| `fgetpos` | `int fgetpos(FILE *f, fpos_t *pos)` | 获取文件位置存入 `fpos_t` 不透明对象 |
| `fsetpos` | `int fsetpos(FILE *f, const fpos_t *pos)` | 从 `fpos_t` 不透明对象恢复文件位置 |
| `rewind` | `void rewind(FILE *f)` | 回绕到文件起始并清除 EOF/错误标志 |

### POSIX 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `fseeko` | `int fseeko(FILE *f, off_t offset, int whence)` | 设置文件位置指示符（`off_t` 偏移，支持大文件） |
| `ftello` | `off_t ftello(FILE *f)` | 获取当前文件位置（返回 `off_t`，支持大文件） |

---

## 9. 流状态

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `feof` | `int feof(FILE *f)` | 测试流文件结束指示符（非零 = 已到文件尾） |
| `ferror` | `int ferror(FILE *f)` | 测试流错误指示符（非零 = 有错误） |
| `clearerr` | `void clearerr(FILE *f)` | 清除流的 EOF 和错误指示符 |
| `fflush` | `int fflush(FILE *f)` | 刷新输出缓冲区到内核（null 参数刷新所有流） |

---

## 10. 流锁定（POSIX）

| 符号 | 签名 | 说明 |
|------|------|------|
| `flockfile` | `void flockfile(FILE *f)` | 获取 FILE 关联的内部递归锁（阻塞式） |
| `ftrylockfile` | `int ftrylockfile(FILE *f)` | 尝试获取 FILE 关联的内部递归锁（非阻塞式，成功返回 0） |
| `funlockfile` | `void funlockfile(FILE *f)` | 释放 FILE 关联的内部递归锁 |

---

## 11. 免锁 I/O 版本

### POSIX 标准免锁函数

| 符号 | 签名 | 说明 |
|------|------|------|
| `getc_unlocked` | `int getc_unlocked(FILE *f)` | 免锁版 getc |
| `getchar_unlocked` | `int getchar_unlocked(void)` | 免锁版 getchar |
| `putc_unlocked` | `int putc_unlocked(int c, FILE *f)` | 免锁版 putc |
| `putchar_unlocked` | `int putchar_unlocked(int c)` | 免锁版 putchar |

### GNU/BSD 免锁函数

| 符号 | 签名 | 说明 |
|------|------|------|
| `fgetc_unlocked` | `int fgetc_unlocked(FILE *f)` | 免锁版 fgetc |
| `fputc_unlocked` | `int fputc_unlocked(int c, FILE *f)` | 免锁版 fputc |
| `fflush_unlocked` | `int fflush_unlocked(FILE *f)` | 免锁版 fflush |
| `fread_unlocked` | `size_t fread_unlocked(void *p, size_t sz, size_t n, FILE *f)` | 免锁版 fread |
| `fwrite_unlocked` | `size_t fwrite_unlocked(const void *p, size_t sz, size_t n, FILE *f)` | 免锁版 fwrite |
| `clearerr_unlocked` | `void clearerr_unlocked(FILE *f)` | 免锁版 clearerr |
| `feof_unlocked` | `int feof_unlocked(FILE *f)` | 免锁版 feof |
| `ferror_unlocked` | `int ferror_unlocked(FILE *f)` | 免锁版 ferror |
| `fileno_unlocked` | `int fileno_unlocked(FILE *f)` | 免锁版 fileno |

### GNU 免锁函数

| 符号 | 签名 | 说明 |
|------|------|------|
| `fgets_unlocked` | `char *fgets_unlocked(char *s, int n, FILE *f)` | 免锁版 fgets |
| `fputs_unlocked` | `int fputs_unlocked(const char *s, FILE *f)` | 免锁版 fputs |

---

## 12. 文件描述符

### POSIX 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `fileno` | `int fileno(FILE *f)` | 获取与 FILE 流关联的底层文件描述符 |
| `fdopen` | `FILE *fdopen(int fd, const char *mode)` | 从已有文件描述符创建 FILE 流 |

---

## 13. 动态行读取（POSIX）

| 符号 | 签名 | 说明 |
|------|------|------|
| `getdelim` | `ssize_t getdelim(char **linep, size_t *np, int delim, FILE *f)` | 读取至分隔符的动态分配行 |
| `getline` | `ssize_t getline(char **linep, size_t *np, FILE *f)` | 读取至 `\n` 的动态分配行（等价 `getdelim(linep, np, '\n', f)`） |

---

## 14. GNU/BSD 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `fgetln` | `char *fgetln(FILE *f, size_t *lenp)` | 零拷贝返回 FILE 缓冲区中下一行指针及长度 |
| `getw` | `int getw(FILE *f)` | 从流读取一个 `int`（二进制） |
| `putw` | `int putw(int w, FILE *f)` | 向流写入一个 `int`（二进制） |

---

## 15. 内存流（POSIX）

| 符号 | 签名 | 说明 |
|------|------|------|
| `fmemopen` | `FILE *fmemopen(void *buf, size_t size, const char *mode)` | 在内存缓冲区上打开流 |
| `open_memstream` | `FILE *open_memstream(char **bufp, size_t *sizep)` | 打开动态增长的内存流（写入时自动扩展） |

---

## 16. Cookie 流（GNU）

| 符号 | 签名 | 说明 |
|------|------|------|
| `fopencookie` | `FILE *fopencookie(void *cookie, const char *mode, cookie_io_functions_t funcs)` | 以自定义回调函数集打开流 |

---

## 17. 终端/用户标识

### POSIX 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `ctermid` | `char *ctermid(char *s)` | 返回当前进程控制终端的路径名 |

### GNU/BSD 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `cuserid` | `char *cuserid(char *s)` | 返回当前用户的登录名（已废弃） |

---

## 18. 其他

### GNU/BSD 扩展

| 符号 | 签名 | 说明 |
|------|------|------|
| `tempnam` | `char *tempnam(const char *dir, const char *pfx)` | 在指定目录生成带前缀的临时文件名 |

### ISO C 标准

| 符号 | 签名 | 说明 |
|------|------|------|
| `perror` | `void perror(const char *s)` | 向 stderr 输出当前 `errno` 对应的错误消息（前缀 `s`） |

---

## 排除说明

以下类别的符号**不出现在**本文件中，它们属于内部实现细节：

- 所有 `__` 前缀的内部函数（如 `__stdio_read`、`__overflow`、`__fdopen` 等）
- `_IO_*` 前缀的 glibc 兼容弱别名（如 `_IO_getc`、`_IO_feof_unlocked` 等）
- `__isoc99_*` 前缀的 C99 标准兼容弱别名（如 `__isoc99_scanf` 等）
- `hidden` 可见性的内部符号（内部引擎函数、静态辅助函数）
- 内部静态变量及结构体实例（如 `__stdin_FILE`、`__stdout_FILE`、`__stderr_FILE` 等）
- 内部弱别名（如 `__getdelim`、`__fseeko`、`__ftello` 等）
- 宽字符 I/O 函数（声明在 `<wchar.h>` 而非 `<stdio.h>`，如 `fgetwc`、`fwprintf` 等）
- `<wchar.h>` 中的函数（如 `open_wmemstream`、`fwide` 等）
