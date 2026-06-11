# stdio 模块 — 对外导出 API 汇总 (Rust 接口设计)

本文件记录 `rusl-stdio` crate 中所有对用户可见的公开接口，对应 musl `<stdio.h>` 中声明的用户可调用符号。内部实现符号（`__` 前缀、`pub(crate)` 模块、弱别名等）不在此列，详见各具体 spec 文件。

rusl 是 `#![no_std]` 项目，所有类型来自 `core::ffi`。对外导出接口为 `pub extern "C" fn`（safe 调用），内部 unsafe 操作封装在函数体中。

---

## 宏常量

C 侧 `<stdio.h>` 中通过 `#define` 定义的宏常量，在 Rust 侧以 `pub const` 形式提供。

### 基本常量

| C 宏 | Rust 常量 | 值 | 说明 |
|------|-----------|-----|------|
| `EOF` | `pub const EOF: c_int = -1;` | `-1` | 文件结束/错误返回值 |
| `SEEK_SET` | `pub const SEEK_SET: c_int = 0;` | `0` | 文件定位 — 从文件头开始 |
| `SEEK_CUR` | `pub const SEEK_CUR: c_int = 1;` | `1` | 文件定位 — 从当前位置开始 |
| `SEEK_END` | `pub const SEEK_END: c_int = 2;` | `2` | 文件定位 — 从文件尾开始 |
| `_IOFBF` | `pub const _IOFBF: c_int = 0;` | `0` | 全缓冲模式 |
| `_IOLBF` | `pub const _IOLBF: c_int = 1;` | `1` | 行缓冲模式 |
| `_IONBF` | `pub const _IONBF: c_int = 2;` | `2` | 无缓冲模式 |
| `BUFSIZ` | `pub const BUFSIZ: usize = 1024;` | `1024` | 默认缓冲区大小 |
| `FILENAME_MAX` | `pub const FILENAME_MAX: usize = 4096;` | `4096` | 文件名最大长度 |
| `FOPEN_MAX` | `pub const FOPEN_MAX: c_int = 1000;` | `1000` | 同时打开的最大文件数 |
| `TMP_MAX` | `pub const TMP_MAX: c_int = 10000;` | `10000` | tmpnam 可生成的最大唯一文件名数 |
| `L_tmpnam` | `pub const L_tmpnam: usize = 20;` | `20` | tmpnam 缓冲区所需最小长度 |
| `L_ctermid` | `pub const L_ctermid: usize = 20;` | `20` | ctermid 缓冲区所需最小长度 |
| `P_tmpdir` | `pub const P_tmpdir: &[u8] = b"/tmp\0";` | `"/tmp"` | 默认临时文件目录路径（null 结尾字节串） |
| `L_cuserid` | `pub const L_cuserid: usize = 20;` | `20` | cuserid 缓冲区所需最小长度 |
| `RENAME_NOREPLACE` | `pub const RENAME_NOREPLACE: c_uint = 1 << 0;` | `1` | renameat2 标志 — 目标存在则失败 |
| `RENAME_EXCHANGE` | `pub const RENAME_EXCHANGE: c_uint = 1 << 1;` | `2` | renameat2 标志 — 原子交换 |
| `RENAME_WHITEOUT` | `pub const RENAME_WHITEOUT: c_uint = 1 << 2;` | `4` | renameat2 标志 — 白化源文件 |

> **注意**: C 中的 `NULL` 宏在 Rust 中直接用 `core::ptr::null()` / `core::ptr::null_mut()` 替代，不提供独立的 `NULL` 常量。

### _LARGEFILE64_SOURCE 兼容别名

在 64 位平台（如 x86_64）上 `off_t` 和 `fpos_t` 天然为 64 位，以下别名通过 Rust 类型别名或重导出实现：

| C 宏 | Rust 对应 |
|------|----------|
| `#define tmpfile64 tmpfile` | `pub use tmpfile as tmpfile64;` (函数重导出) |
| `#define fopen64 fopen` | `pub use fopen as fopen64;` |
| `#define freopen64 freopen` | `pub use freopen as freopen64;` |
| `#define fseeko64 fseeko` | `pub use fseeko as fseeko64;` |
| `#define ftello64 ftello` | `pub use ftello as ftello64;` |
| `#define fgetpos64 fgetpos` | `pub use fgetpos as fgetpos64;` |
| `#define fsetpos64 fsetpos` | `pub use fsetpos as fsetpos64;` |
| `#define fpos64_t fpos_t` | `pub type fpos64_t = fpos_t;` |
| `#define off64_t off_t` | `pub type off64_t = off_t;` |

---

## 类型定义

| C 类型 | Rust 类型 | 说明 |
|--------|----------|------|
| `FILE` (`struct _IO_FILE`) | `#[repr(C)] pub struct FILE { ... }` | 标准 I/O 流对象（完整定义见 `stdio_impl.rs`） |
| `fpos_t` | `#[repr(C)] pub union fpos_t { __opaque: [u8; 16], __lldata: i64, __align: f64 }` | 文件位置不透明类型（ISO C） |
| `cookie_read_function_t` | `pub type cookie_read_function_t = Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>;` | Cookie 流读取回调（GNU） |
| `cookie_write_function_t` | `pub type cookie_write_function_t = Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>;` | Cookie 流写入回调（GNU） |
| `cookie_seek_function_t` | `pub type cookie_seek_function_t = Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>;` | Cookie 流定位回调（GNU） |
| `cookie_close_function_t` | `pub type cookie_close_function_t = Option<unsafe extern "C" fn(*mut c_void) -> c_int>;` | Cookie 流关闭回调（GNU） |
| `cookie_io_functions_t` | `#[repr(C)] pub struct cookie_io_functions_t { pub read: cookie_read_function_t, pub write: cookie_write_function_t, pub seek: cookie_seek_function_t, pub close: cookie_close_function_t }` | Cookie 流回调集（GNU） |
| `off_t` | `pub type off_t = i64;` | 文件偏移类型（x86_64 上为 64 位） |
| `ssize_t` | `pub type ssize_t = isize;` | 有符号 size 类型（POSIX） |
| `VaList` | `#[repr(C)] pub struct VaList { ... }` | x86_64 System V AMD64 ABI va_list（定义见 `stdio_impl.rs`） |

> **设计说明**: Rust 侧的 `FILE` 结构体布局必须与 musl `struct _IO_FILE` 完全一致，使用 `#[repr(C)]` 确保 ABI 兼容。内部函数指针字段使用 `Option<unsafe extern "C" fn(...)>` 表示可空的 C 函数指针。

---

## 标准流对象

```c
// C 侧
extern FILE *const stdin;
extern FILE *const stdout;
extern FILE *const stderr;
```

```rust
// Rust 侧 — 声明于 extern "C" 块，编译时由链接器解析到 musl 或 rusl 提供的符号
extern "C" {
    pub static stdin: *mut FILE;
    pub static stdout: *mut FILE;
    pub static stderr: *mut FILE;
}
```

| 符号 | Rust 类型 | 标准 | 说明 |
|------|----------|------|------|
| `stdin` | `*mut FILE` | ISO C | 标准输入流（文件描述符 0） |
| `stdout` | `*mut FILE` | ISO C | 标准输出流（文件描述符 1） |
| `stderr` | `*mut FILE` | ISO C | 标准错误输出流（文件描述符 2） |

> **注意事项**: 标准流对象是可变静态变量（`*mut FILE`），在多线程环境中通过内部锁保护。Rust 侧不提供不可变引用包装，保持与 C 侧一致的 `*mut` 语义。

---

## 1. 文件操作

### ISO C 标准

**`fopen`**

```c
// C 签名
FILE *fopen(const char *path, const char *mode);
```

```rust
// Rust 签名
pub extern "C" fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
```

**`freopen`**

```c
// C 签名
FILE *freopen(const char *path, const char *mode, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn freopen(path: *const c_char, mode: *const c_char, f: *mut FILE) -> *mut FILE;
```

**`fclose`**

```c
// C 签名
int fclose(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fclose(f: *mut FILE) -> c_int;
```

**`remove`**

```c
// C 签名
int remove(const char *path);
```

```rust
// Rust 签名
pub extern "C" fn remove(path: *const c_char) -> c_int;
```

**`rename`**

```c
// C 签名
int rename(const char *old, const char *new);
```

```rust
// Rust 签名
pub extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int;
```

**`tmpfile`**

```c
// C 签名
FILE *tmpfile(void);
```

```rust
// Rust 签名
pub extern "C" fn tmpfile() -> *mut FILE;
```

**`tmpnam`**

```c
// C 签名
char *tmpnam(char *s);
```

```rust
// Rust 签名
pub extern "C" fn tmpnam(s: *mut c_char) -> *mut c_char;
```

### POSIX 扩展

**`renameat`**

```c
// C 签名
int renameat(int oldfd, const char *old, int newfd, const char *new);
```

```rust
// Rust 签名
pub extern "C" fn renameat(oldfd: c_int, old: *const c_char, newfd: c_int, new: *const c_char) -> c_int;
```

**`popen`**

```c
// C 签名
FILE *popen(const char *cmd, const char *mode);
```

```rust
// Rust 签名
pub extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE;
```

**`pclose`**

```c
// C 签名
int pclose(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn pclose(f: *mut FILE) -> c_int;
```

### GNU 扩展

**`renameat2`**

```c
// C 签名
int renameat2(int oldfd, const char *old, int newfd, const char *new, unsigned flags);
```

```rust
// Rust 签名
pub extern "C" fn renameat2(oldfd: c_int, old: *const c_char, newfd: c_int, new: *const c_char, flags: c_uint) -> c_int;
```

---

## 2. 流缓冲

### ISO C 标准

**`setbuf`**

```c
// C 签名
void setbuf(FILE *f, char *buf);
```

```rust
// Rust 签名
pub extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char);
```

**`setvbuf`**

```c
// C 签名
int setvbuf(FILE *f, char *buf, int mode, size_t size);
```

```rust
// Rust 签名
pub extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
```

### GNU/BSD 扩展

**`setbuffer`**

```c
// C 签名
void setbuffer(FILE *f, char *buf, size_t size);
```

```rust
// Rust 签名
pub extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: usize);
```

**`setlinebuf`**

```c
// C 签名
void setlinebuf(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn setlinebuf(f: *mut FILE);
```

---

## 3. 格式化输出（printf 家族）

### ISO C 标准

### `printf` / `fprintf` / `sprintf` / `snprintf`（可变参数）

```c
// C 签名
int printf(const char *fmt, ...);
int fprintf(FILE *f, const char *fmt, ...);
int sprintf(char *s, const char *fmt, ...);
int snprintf(char *s, size_t n, const char *fmt, ...);
```

```rust
// Rust 签名 — 可变参数函数在 Rust 中无法用 extern "C" fn 定义函数体
// 采用 extern "C" 声明块，由 C 侧 thin wrapper 或链接器提供实际实现
extern "C" {
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}
```

> **Rust 实现策略**: 上述可变参数函数由 C 源码 thin wrapper 实现（调用对应 `v*` 版本），与 musl 原始设计一致。

### `va_list` 版本（可在 Rust 中实现）

**`vprintf` / `vfprintf` / `vsprintf` / `vsnprintf`**

```c
// C 签名
int vprintf(const char *fmt, va_list ap);
int vfprintf(FILE *f, const char *fmt, va_list ap);
int vsprintf(char *s, const char *fmt, va_list ap);
int vsnprintf(char *s, size_t n, const char *fmt, va_list ap);
```

```rust
// Rust 签名 — va_list 在 x86_64 上以 VaList 结构体按值传递
// 使用 #[no_mangle] extern "C" 保持 ABI 兼容，函数体在 Rust 中安全实现
pub extern "C" fn vprintf(fmt: *const c_char, ap: VaList) -> c_int;
pub extern "C" fn vfprintf(f: *mut FILE, fmt: *const c_char, ap: VaList) -> c_int;
pub extern "C" fn vsprintf(s: *mut c_char, fmt: *const c_char, ap: VaList) -> c_int;
pub extern "C" fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: VaList) -> c_int;
```

### POSIX 扩展

**`dprintf` / `vdprintf`**

```c
// C 签名
int dprintf(int fd, const char *fmt, ...);
int vdprintf(int fd, const char *fmt, va_list ap);
```

```rust
// Rust 签名
extern "C" {
    pub fn dprintf(fd: c_int, fmt: *const c_char, ...) -> c_int;
}
pub extern "C" fn vdprintf(fd: c_int, fmt: *const c_char, ap: VaList) -> c_int;
```

### GNU/BSD 扩展

**`asprintf` / `vasprintf`**

```c
// C 签名
int asprintf(char **sp, const char *fmt, ...);
int vasprintf(char **sp, const char *fmt, va_list ap);
```

```rust
// Rust 签名
extern "C" {
    pub fn asprintf(sp: *mut *mut c_char, fmt: *const c_char, ...) -> c_int;
}
pub extern "C" fn vasprintf(sp: *mut *mut c_char, fmt: *const c_char, ap: VaList) -> c_int;
```

---

## 4. 格式化输入（scanf 家族）

### ISO C 标准

```c
// C 签名（可变参数）
int scanf(const char *fmt, ...);
int fscanf(FILE *f, const char *fmt, ...);
int sscanf(const char *s, const char *fmt, ...);
```

```rust
// Rust 签名 — 可变参数，与 printf 家族处理方式一致
extern "C" {
    pub fn scanf(fmt: *const c_char, ...) -> c_int;
    pub fn fscanf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
}
```

```c
// C 签名（va_list 版本）
int vscanf(const char *fmt, va_list ap);
int vfscanf(FILE *f, const char *fmt, va_list ap);
int vsscanf(const char *s, const char *fmt, va_list ap);
```

```rust
// Rust 签名
pub extern "C" fn vscanf(fmt: *const c_char, ap: VaList) -> c_int;
pub extern "C" fn vfscanf(f: *mut FILE, fmt: *const c_char, ap: VaList) -> c_int;
pub extern "C" fn vsscanf(s: *const c_char, fmt: *const c_char, ap: VaList) -> c_int;
```

---

## 5. 字符 I/O

### ISO C 标准

```c
// C 签名
int fgetc(FILE *f);
int fputc(int c, FILE *f);
int getc(FILE *f);
int putc(int c, FILE *f);
int getchar(void);
int putchar(int c);
int ungetc(int c, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fgetc(f: *mut FILE) -> c_int;
pub extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int;
pub extern "C" fn getc(f: *mut FILE) -> c_int;
pub extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int;
pub extern "C" fn getchar() -> c_int;
pub extern "C" fn putchar(c: c_int) -> c_int;
pub extern "C" fn ungetc(c: c_int, f: *mut FILE) -> c_int;
```

> **注意**: C 中 `getc`/`putc` 通常作为宏实现（展开为内联操作），Rust 侧直接提供函数实现，内联优化由编译器 `#[inline]` 属性控制。

---

## 6. 字符串 I/O

### ISO C 标准

```c
// C 签名
char *fgets(char *s, int n, FILE *f);
int fputs(const char *s, FILE *f);
int puts(const char *s);
char *gets(char *s);  // C11 移除，视为已废弃
```

```rust
// Rust 签名
pub extern "C" fn fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;
pub extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int;
pub extern "C" fn puts(s: *const c_char) -> c_int;
pub extern "C" fn gets(s: *mut c_char) -> *mut c_char;  // 已废弃
```

---

## 7. 块 I/O

### ISO C 标准

```c
// C 签名
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *f);
size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fread(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
pub extern "C" fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
```

---

## 8. 文件定位

### ISO C 标准

```c
// C 签名
int fseek(FILE *f, long offset, int whence);
long ftell(FILE *f);
int fgetpos(FILE *f, fpos_t *pos);
int fsetpos(FILE *f, const fpos_t *pos);
void rewind(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fseek(f: *mut FILE, offset: c_long, whence: c_int) -> c_int;
pub extern "C" fn ftell(f: *mut FILE) -> c_long;
pub extern "C" fn fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int;
pub extern "C" fn fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int;
pub extern "C" fn rewind(f: *mut FILE);
```

### POSIX 扩展

```c
// C 签名
int fseeko(FILE *f, off_t offset, int whence);
off_t ftello(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fseeko(f: *mut FILE, offset: off_t, whence: c_int) -> c_int;
pub extern "C" fn ftello(f: *mut FILE) -> off_t;
```

---

## 9. 流状态

### ISO C 标准

```c
// C 签名
int feof(FILE *f);
int ferror(FILE *f);
void clearerr(FILE *f);
int fflush(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn feof(f: *mut FILE) -> c_int;
pub extern "C" fn ferror(f: *mut FILE) -> c_int;
pub extern "C" fn clearerr(f: *mut FILE);
pub extern "C" fn fflush(f: *mut FILE) -> c_int;
```

> **fflush 语义**: 参数 `f` 为 `null` 时刷新所有输出流。

---

## 10. 流锁定（POSIX）

```c
// C 签名
void flockfile(FILE *f);
int ftrylockfile(FILE *f);
void funlockfile(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn flockfile(f: *mut FILE);
pub extern "C" fn ftrylockfile(f: *mut FILE) -> c_int;
pub extern "C" fn funlockfile(f: *mut FILE);
```

> **Rust 实现说明**: 内部使用原子操作或互斥原语实现递归锁，`FILE.flock` / `FILE.lockcount` 字段用于跟踪锁状态。线程锁支持定义于 `rusl-stdio` 的 `__lockfile` 模块。

---

## 11. 免锁 I/O 版本

### POSIX 标准免锁函数

```c
// C 签名
int getc_unlocked(FILE *f);
int getchar_unlocked(void);
int putc_unlocked(int c, FILE *f);
int putchar_unlocked(int c);
```

```rust
// Rust 签名
pub extern "C" fn getc_unlocked(f: *mut FILE) -> c_int;
pub extern "C" fn getchar_unlocked() -> c_int;
pub extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
pub extern "C" fn putchar_unlocked(c: c_int) -> c_int;
```

### GNU/BSD 免锁函数

```c
// C 签名
int fgetc_unlocked(FILE *f);
int fputc_unlocked(int c, FILE *f);
int fflush_unlocked(FILE *f);
size_t fread_unlocked(void *p, size_t sz, size_t n, FILE *f);
size_t fwrite_unlocked(const void *p, size_t sz, size_t n, FILE *f);
void clearerr_unlocked(FILE *f);
int feof_unlocked(FILE *f);
int ferror_unlocked(FILE *f);
int fileno_unlocked(FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int;
pub extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int;
pub extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int;
pub extern "C" fn fread_unlocked(p: *mut c_void, sz: usize, n: usize, f: *mut FILE) -> usize;
pub extern "C" fn fwrite_unlocked(p: *const c_void, sz: usize, n: usize, f: *mut FILE) -> usize;
pub extern "C" fn clearerr_unlocked(f: *mut FILE);
pub extern "C" fn feof_unlocked(f: *mut FILE) -> c_int;
pub extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int;
pub extern "C" fn fileno_unlocked(f: *mut FILE) -> c_int;
```

> **弱别名关系**: 在 musl 中 `fwrite_unlocked` / `fread_unlocked` 等与对应的加锁版本通过弱别名共享同一实现（加锁版本在函数名前添加 `__lockfile` 调用后跳转到免锁实现）。Rust 侧通过 `pub(crate)` 内部共享函数实现，两个公开接口分别提供加锁/免锁包装。

### GNU 免锁函数

```c
// C 签名
char *fgets_unlocked(char *s, int n, FILE *f);
int fputs_unlocked(const char *s, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;
pub extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int;
```

---

## 12. 文件描述符

### POSIX 扩展

```c
// C 签名
int fileno(FILE *f);
FILE *fdopen(int fd, const char *mode);
```

```rust
// Rust 签名
pub extern "C" fn fileno(f: *mut FILE) -> c_int;
pub extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
```

---

## 13. 动态行读取（POSIX）

```c
// C 签名
ssize_t getdelim(char **linep, size_t *np, int delim, FILE *f);
ssize_t getline(char **linep, size_t *np, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn getdelim(linep: *mut *mut c_char, np: *mut usize, delim: c_int, f: *mut FILE) -> isize;
pub extern "C" fn getline(linep: *mut *mut c_char, np: *mut usize, f: *mut FILE) -> isize;
```

---

## 14. GNU/BSD 扩展

```c
// C 签名
char *fgetln(FILE *f, size_t *lenp);
int getw(FILE *f);
int putw(int w, FILE *f);
```

```rust
// Rust 签名
pub extern "C" fn fgetln(f: *mut FILE, lenp: *mut usize) -> *mut c_char;
pub extern "C" fn getw(f: *mut FILE) -> c_int;
pub extern "C" fn putw(w: c_int, f: *mut FILE) -> c_int;
```

---

## 15. 内存流（POSIX）

```c
// C 签名
FILE *fmemopen(void *buf, size_t size, const char *mode);
FILE *open_memstream(char **bufp, size_t *sizep);
```

```rust
// Rust 签名
pub extern "C" fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE;
pub extern "C" fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut FILE;
```

---

## 16. Cookie 流（GNU）

```c
// C 签名
FILE *fopencookie(void *cookie, const char *mode, cookie_io_functions_t funcs);
```

```rust
// Rust 签名
pub extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    funcs: cookie_io_functions_t,
) -> *mut FILE;
```

---

## 17. 终端/用户标识

### POSIX 扩展

```c
// C 签名
char *ctermid(char *s);
```

```rust
// Rust 签名
pub extern "C" fn ctermid(s: *mut c_char) -> *mut c_char;
```

### GNU/BSD 扩展

```c
// C 签名
char *cuserid(char *s);
```

```rust
// Rust 签名
pub extern "C" fn cuserid(s: *mut c_char) -> *mut c_char;
```

---

## 18. 其他

### GNU/BSD 扩展

```c
// C 签名
char *tempnam(const char *dir, const char *pfx);
```

```rust
// Rust 签名
pub extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char;
```

### ISO C 标准

```c
// C 签名
void perror(const char *s);
```

```rust
// Rust 签名
pub extern "C" fn perror(s: *const c_char);
```

> **Rust 实现说明**: `perror` 需要读取全局 `errno`。rusl 使用 `rusl-errno` crate 提供的 `__errno_location()` 获取 errno 地址。

---

## 模块组织

Rust 侧 stdio 接口按以下 crate 层级组织：

```
rusl-stdio/                     # #![no_std] stdio 实现 crate
  ├── lib.rs                    # crate 入口，声明模块、pub use 重导出
  ├── stdio_impl.rs             # FILE 结构体、F_* 标志、VaList、常量（pub(crate) 或 pub）
  ├── fopen.rs                  # 文件操作
  ├── fclose.rs                 # 文件关闭
  ├── fread.rs / fwrite.rs      # 块 I/O
  ├── fprintf.rs / printf.rs    # 格式化输出引擎
  ├── fscanf.rs / scanf.rs      # 格式化输入引擎
  ├── fgetc.rs / fputc.rs       # 字符 I/O
  ├── ...                       # 其他模块

rusl-main/
  └── src/api/stdio.rs          # 对外 API 声明（c-test 模式下声明 extern 符号；
                                #  非 c-test 模式下 pub use rusl_stdio::* 重导出）
```

---

## 排除说明

以下类别的符号**不出现在**本文件中，它们属于内部实现细节：

- 所有 `__` 前缀的内部函数（如 `__stdio_read`、`__overflow`、`__fdopen`、`__fwritex`、`__towrite`、`__uflow`、`__toread` 等）—— 在 Rust 侧定义为 `pub(crate)` 模块，仅 crate 内部可见
- `_IO_*` 前缀的 glibc 兼容弱别名（如 `_IO_getc`、`_IO_feof_unlocked` 等）—— Rust 侧不提供（仅 glibc ABI 兼容需要）
- `__isoc99_*` 前缀的 C99 标准兼容弱别名（如 `__isoc99_scanf` 等）—— Rust 侧不提供（仅 C 编译器需要）
- `hidden` 可见性的内部符号（内部引擎函数、静态辅助函数）—— 对应 Rust 的 `pub(crate)` 或更高限制的可见性
- 内部静态变量及结构体实例（如 `__stdin_FILE`、`__stdout_FILE`、`__stderr_FILE` 等）—— 由 Rust 模块级 `static` 实现，不对外暴露
- 内部弱别名（如 `__getdelim`、`__fseeko`、`__ftello` 等）—— musl 中 `__` 版本为主实现。rusl 中同时在 `extern "C"` 下导出 `__xxx` 与 `xxx` 两个符号（符合 CLAUDE.md 要求），但只有 `xxx` 视为对外公开接口
- 宽字符 I/O 函数（如 `fgetwc`、`fwprintf` 等，声明域在 `<wchar.h>` 而非 `<stdio.h>`）—— 在 Rust 侧划入独立的 `rusl-wchar` 或 `rusl-stdio` 的 `wchar` 子模块，不在本 stdio export 列表
- `<wchar.h>` 中的内存流（如 `open_wmemstream`）—— 属于宽字符域，同样排除
- `__ofl_head` 等全局链表变量 —— 由 `rusl-stdio::ofl` 模块以 Rust 静态变量管理
