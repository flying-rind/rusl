//! stdio — 标准 I/O 对外 API 声明
//!
//! 本模块仅在未启用 `rusl` feature 时编译, 通过 `extern "C"` 声明
//! musl libc 的标准 I/O 符号。启用 `rusl` feature 时,
//! api/mod.rs 直接从 rusl_stdio crate 重新导出。

use core::ffi::{c_char, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// 类型定义
// ---------------------------------------------------------------------------

/// FILE — 对应 musl `struct _IO_FILE`。布局须与 rusl-stdio 一致。
#[repr(C)]
pub struct FILE {
    pub flags: c_uint,
    pub rpos: *mut u8,
    pub rend: *mut u8,
    pub close: Option<unsafe extern "C" fn(*mut FILE) -> c_int>,
    pub wend: *mut u8,
    pub wpos: *mut u8,
    pub mustbezero_1: *mut u8,
    pub wbase: *mut u8,
    pub read: Option<unsafe extern "C" fn(*mut FILE, *mut u8, usize) -> usize>,
    pub write: Option<unsafe extern "C" fn(*mut FILE, *const u8, usize) -> usize>,
    pub seek: Option<unsafe extern "C" fn(*mut FILE, i64, c_int) -> i64>,
    pub buf: *mut u8,
    pub buf_size: usize,
    pub prev: *mut FILE,
    pub next: *mut FILE,
    pub fd: c_int,
    pub pipe_pid: c_int,
    pub lockcount: c_long,
    pub mode: c_int,
    pub lock: c_int,
    pub lbf: c_int,
    pub cookie: *mut c_void,
    pub off: i64,
    pub getln_buf: *mut c_char,
    pub mustbezero_2: *mut c_void,
    pub shend: *mut u8,
    pub shlim: i64,
    pub shcnt: i64,
    pub prev_locked: *mut FILE,
    pub next_locked: *mut FILE,
    pub locale: *mut c_void,
}

/// x86_64 va_list — System V AMD64 ABI, 24 字节按值传递
#[repr(C)]
pub struct VaList {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

/// fpos_t — 文件位置类型（musl 中为 i64）
pub type fpos_t = i64;

/// off_t — 文件偏移量类型（x86-64 Linux 为 i64）
pub type off_t = i64;

/// cookie_io_functions_t — fopencookie 回调函数集合
#[repr(C)]
pub struct cookie_io_functions_t {
    pub read: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    pub write: Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>,
    pub seek: Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>,
    pub close: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------

pub const EOF: c_int = -1;

// ---------------------------------------------------------------------------
// 标准流（musl libc 导出符号）
// ---------------------------------------------------------------------------

extern "C" {
    pub static stdin: *mut FILE;
    pub static stdout: *mut FILE;
    pub static stderr: *mut FILE;
}

// ---------------------------------------------------------------------------
// 基础 I/O
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fwrite"]
    fn musl_fwrite(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    #[link_name = "fread"]
    fn musl_fread(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    #[link_name = "fgets"]
    fn musl_fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;
    #[link_name = "fputs"]
    fn musl_fputs(s: *const c_char, f: *mut FILE) -> c_int;
    #[link_name = "fflush"]
    fn musl_fflush(f: *mut FILE) -> c_int;
    #[link_name = "fclose"]
    fn musl_fclose(f: *mut FILE) -> c_int;
    #[link_name = "fopen"]
    fn musl_fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE;
}

pub extern "C" fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unsafe { musl_fwrite(ptr, size, nmemb, f) }
}
pub extern "C" fn fread(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unsafe { musl_fread(ptr, size, nmemb, f) }
}
pub extern "C" fn fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char {
    unsafe { musl_fgets(s, n, f) }
}
pub extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int {
    unsafe { musl_fputs(s, f) }
}
pub extern "C" fn fflush(f: *mut FILE) -> c_int {
    unsafe { musl_fflush(f) }
}
pub extern "C" fn fclose(f: *mut FILE) -> c_int {
    unsafe { musl_fclose(f) }
}
pub extern "C" fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE {
    unsafe { musl_fopen(filename, mode) }
}

// ---------------------------------------------------------------------------
// 字符 I/O
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fgetc"]
    fn musl_fgetc(f: *mut FILE) -> c_int;
    #[link_name = "fputc"]
    fn musl_fputc(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "getc"]
    fn musl_getc(f: *mut FILE) -> c_int;
    #[link_name = "putc"]
    fn musl_putc(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "getchar"]
    fn musl_getchar() -> c_int;
    #[link_name = "putchar"]
    fn musl_putchar(c: c_int) -> c_int;
    #[link_name = "puts"]
    fn musl_puts(s: *const c_char) -> c_int;
    #[link_name = "gets"]
    fn musl_gets(s: *mut c_char) -> *mut c_char;
}

pub extern "C" fn fgetc(f: *mut FILE) -> c_int { unsafe { musl_fgetc(f) } }
pub extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_fputc(c, f) } }
pub extern "C" fn getc(f: *mut FILE) -> c_int { unsafe { musl_getc(f) } }
pub extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_putc(c, f) } }
pub extern "C" fn getchar() -> c_int { unsafe { musl_getchar() } }
pub extern "C" fn putchar(c: c_int) -> c_int { unsafe { musl_putchar(c) } }
pub extern "C" fn puts(s: *const c_char) -> c_int { unsafe { musl_puts(s) } }
pub extern "C" fn gets(s: *mut c_char) -> *mut c_char { unsafe { musl_gets(s) } }

// ---------------------------------------------------------------------------
// 格式化 — va_list 版本（vfprintf / vsnprintf / vsprintf / vprintf）
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "vfprintf"]
    fn musl_vfprintf(f: *mut FILE, fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vsnprintf"]
    fn musl_vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vsprintf"]
    fn musl_vsprintf(s: *mut c_char, fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vprintf"]
    fn musl_vprintf(fmt: *const c_char, ap: VaList) -> c_int;
}

pub extern "C" fn vfprintf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vfprintf(f, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vsnprintf(s: *mut c_char, n: usize, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vsnprintf(s, n, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vsprintf(s: *mut c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vsprintf(s, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vprintf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vprintf(fmt, core::ptr::read(ap)) }
}

// ---------------------------------------------------------------------------
// 格式化 — 可变参数版本（printf / fprintf / sprintf / snprintf）
// ---------------------------------------------------------------------------

extern "C" {
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// 格式化输出扩展 — va_list 版本
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "vdprintf"]
    fn musl_vdprintf(fd: c_int, fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vasprintf"]
    fn musl_vasprintf(s: *mut *mut c_char, fmt: *const c_char, ap: VaList) -> c_int;
}

pub extern "C" fn vdprintf(fd: c_int, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vdprintf(fd, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vasprintf(s: *mut *mut c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vasprintf(s, fmt, core::ptr::read(ap)) }
}

// ---------------------------------------------------------------------------
// 格式化输出扩展 — 可变参数版本
// ---------------------------------------------------------------------------

extern "C" {
    pub fn dprintf(fd: c_int, fmt: *const c_char, ...) -> c_int;
    pub fn asprintf(s: *mut *mut c_char, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// 格式化输入 — va_list 版本
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "vfscanf"]
    fn musl_vfscanf(f: *mut FILE, fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vscanf"]
    fn musl_vscanf(fmt: *const c_char, ap: VaList) -> c_int;
    #[link_name = "vsscanf"]
    fn musl_vsscanf(s: *const c_char, fmt: *const c_char, ap: VaList) -> c_int;
}

pub extern "C" fn vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vfscanf(f, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vscanf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vscanf(fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe { musl_vsscanf(s, fmt, core::ptr::read(ap)) }
}

// ---------------------------------------------------------------------------
// 格式化输入 — 可变参数版本
// ---------------------------------------------------------------------------

extern "C" {
    pub fn scanf(fmt: *const c_char, ...) -> c_int;
    pub fn fscanf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// 流状态
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "feof"]
    fn musl_feof(f: *mut FILE) -> c_int;
    #[link_name = "ferror"]
    fn musl_ferror(f: *mut FILE) -> c_int;
    #[link_name = "clearerr"]
    fn musl_clearerr(f: *mut FILE);
    #[link_name = "fileno"]
    fn musl_fileno(f: *mut FILE) -> c_int;
}

pub extern "C" fn feof(f: *mut FILE) -> c_int { unsafe { musl_feof(f) } }
pub extern "C" fn ferror(f: *mut FILE) -> c_int { unsafe { musl_ferror(f) } }
pub extern "C" fn clearerr(f: *mut FILE) { unsafe { musl_clearerr(f) } }
pub extern "C" fn fileno(f: *mut FILE) -> c_int { unsafe { musl_fileno(f) } }

// ---------------------------------------------------------------------------
// 文件定位
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fseek"]
    fn musl_fseek(f: *mut FILE, offset: c_long, whence: c_int) -> c_int;
    #[link_name = "ftell"]
    fn musl_ftell(f: *mut FILE) -> c_long;
    #[link_name = "rewind"]
    fn musl_rewind(f: *mut FILE);
    #[link_name = "fseeko"]
    fn musl_fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int;
    #[link_name = "ftello"]
    fn musl_ftello(f: *mut FILE) -> off_t;
    #[link_name = "fgetpos"]
    fn musl_fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int;
    #[link_name = "fsetpos"]
    fn musl_fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int;
}

pub extern "C" fn fseek(f: *mut FILE, offset: c_long, whence: c_int) -> c_int {
    unsafe { musl_fseek(f, offset, whence) }
}
pub extern "C" fn ftell(f: *mut FILE) -> c_long {
    unsafe { musl_ftell(f) }
}
pub extern "C" fn rewind(f: *mut FILE) {
    unsafe { musl_rewind(f) }
}
pub extern "C" fn fseeko(f: *mut FILE, off: off_t, whence: c_int) -> c_int {
    unsafe { musl_fseeko(f, off, whence) }
}
pub extern "C" fn ftello(f: *mut FILE) -> off_t {
    unsafe { musl_ftello(f) }
}
pub extern "C" fn fgetpos(f: *mut FILE, pos: *mut fpos_t) -> c_int {
    unsafe { musl_fgetpos(f, pos) }
}
pub extern "C" fn fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int {
    unsafe { musl_fsetpos(f, pos) }
}

// ---------------------------------------------------------------------------
// 缓冲控制
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "setbuf"]
    fn musl_setbuf(f: *mut FILE, buf: *mut c_char);
    #[link_name = "setvbuf"]
    fn musl_setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    #[link_name = "setbuffer"]
    fn musl_setbuffer(f: *mut FILE, buf: *mut c_char, size: usize);
    #[link_name = "setlinebuf"]
    fn musl_setlinebuf(f: *mut FILE);
}

pub extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char) {
    unsafe { musl_setbuf(f, buf) }
}
pub extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int {
    unsafe { musl_setvbuf(f, buf, mode, size) }
}
pub extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: usize) {
    unsafe { musl_setbuffer(f, buf, size) }
}
pub extern "C" fn setlinebuf(f: *mut FILE) {
    unsafe { musl_setlinebuf(f) }
}

// ---------------------------------------------------------------------------
// 文件系统操作
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "remove"]
    fn musl_remove(path: *const c_char) -> c_int;
    #[link_name = "rename"]
    fn musl_rename(old: *const c_char, new: *const c_char) -> c_int;
    #[link_name = "tmpfile"]
    fn musl_tmpfile() -> *mut FILE;
    #[link_name = "tmpnam"]
    fn musl_tmpnam(buf: *mut c_char) -> *mut c_char;
    #[link_name = "tempnam"]
    fn musl_tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char;
}

pub extern "C" fn remove(path: *const c_char) -> c_int { unsafe { musl_remove(path) } }
pub extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int { unsafe { musl_rename(old, new) } }
pub extern "C" fn tmpfile() -> *mut FILE { unsafe { musl_tmpfile() } }
pub extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char { unsafe { musl_tmpnam(buf) } }
pub extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char { unsafe { musl_tempnam(dir, pfx) } }

// ---------------------------------------------------------------------------
// 高级流操作
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fmemopen"]
    fn musl_fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE;
    #[link_name = "open_memstream"]
    fn musl_open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut FILE;
    #[link_name = "open_wmemstream"]
    fn musl_open_wmemstream(bufp: *mut *mut c_int, sizep: *mut usize) -> *mut FILE;
    #[link_name = "popen"]
    fn musl_popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE;
    #[link_name = "pclose"]
    fn musl_pclose(f: *mut FILE) -> c_int;
    #[link_name = "fgetln"]
    fn musl_fgetln(f: *mut FILE, plen: *mut usize) -> *mut c_char;
    #[link_name = "getdelim"]
    fn musl_getdelim(lineptr: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize;
    #[link_name = "getline"]
    fn musl_getline(lineptr: *mut *mut c_char, n: *mut usize, f: *mut FILE) -> isize;
}

pub extern "C" fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE {
    unsafe { musl_fmemopen(buf, size, mode) }
}
pub extern "C" fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut FILE {
    unsafe { musl_open_memstream(bufp, sizep) }
}
pub extern "C" fn open_wmemstream(bufp: *mut *mut c_int, sizep: *mut usize) -> *mut FILE {
    unsafe { musl_open_wmemstream(bufp, sizep) }
}
pub extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE {
    unsafe { musl_popen(cmd, mode) }
}
pub extern "C" fn pclose(f: *mut FILE) -> c_int { unsafe { musl_pclose(f) } }
pub extern "C" fn fgetln(f: *mut FILE, plen: *mut usize) -> *mut c_char {
    unsafe { musl_fgetln(f, plen) }
}
pub extern "C" fn getdelim(lineptr: *mut *mut c_char, n: *mut usize, delim: c_int, f: *mut FILE) -> isize {
    unsafe { musl_getdelim(lineptr, n, delim, f) }
}
pub extern "C" fn getline(lineptr: *mut *mut c_char, n: *mut usize, f: *mut FILE) -> isize {
    unsafe { musl_getline(lineptr, n, f) }
}

// ---------------------------------------------------------------------------
// 字符推回
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "ungetc"]
    fn musl_ungetc(c: c_int, f: *mut FILE) -> c_int;
}

pub extern "C" fn ungetc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_ungetc(c, f) } }

// ---------------------------------------------------------------------------
// 宽字符方向
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fwide"]
    fn musl_fwide(f: *mut FILE, mode: c_int) -> c_int;
}

pub extern "C" fn fwide(f: *mut FILE, mode: c_int) -> c_int { unsafe { musl_fwide(f, mode) } }

// ---------------------------------------------------------------------------
// 错误消息
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "perror"]
    fn musl_perror(msg: *const c_char);
}

pub extern "C" fn perror(msg: *const c_char) { unsafe { musl_perror(msg) } }

// ---------------------------------------------------------------------------
// 二进制整数读写
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "getw"]
    fn musl_getw(f: *mut FILE) -> c_int;
    #[link_name = "putw"]
    fn musl_putw(x: c_int, f: *mut FILE) -> c_int;
}

pub extern "C" fn getw(f: *mut FILE) -> c_int { unsafe { musl_getw(f) } }
pub extern "C" fn putw(x: c_int, f: *mut FILE) -> c_int { unsafe { musl_putw(x, f) } }

// ---------------------------------------------------------------------------
// 免锁变体（POSIX.1-2001 _unlocked 扩展）
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fread_unlocked"]
    fn musl_fread_unlocked(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    #[link_name = "fwrite_unlocked"]
    fn musl_fwrite_unlocked(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    #[link_name = "fgets_unlocked"]
    fn musl_fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char;
    #[link_name = "fputs_unlocked"]
    fn musl_fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int;
    #[link_name = "fflush_unlocked"]
    fn musl_fflush_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "fgetc_unlocked"]
    fn musl_fgetc_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "getc_unlocked"]
    fn musl_getc_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "getchar_unlocked"]
    fn musl_getchar_unlocked() -> c_int;
    #[link_name = "fputc_unlocked"]
    fn musl_fputc_unlocked(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "putc_unlocked"]
    fn musl_putc_unlocked(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "putchar_unlocked"]
    fn musl_putchar_unlocked(c: c_int) -> c_int;
    #[link_name = "feof_unlocked"]
    fn musl_feof_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "ferror_unlocked"]
    fn musl_ferror_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "clearerr_unlocked"]
    fn musl_clearerr_unlocked(f: *mut FILE);
    #[link_name = "fileno_unlocked"]
    fn musl_fileno_unlocked(f: *mut FILE) -> c_int;
}

pub extern "C" fn fread_unlocked(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unsafe { musl_fread_unlocked(ptr, size, nmemb, f) }
}
pub extern "C" fn fwrite_unlocked(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize {
    unsafe { musl_fwrite_unlocked(ptr, size, nmemb, f) }
}
pub extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char {
    unsafe { musl_fgets_unlocked(s, n, f) }
}
pub extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int {
    unsafe { musl_fputs_unlocked(s, f) }
}
pub extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int { unsafe { musl_fflush_unlocked(f) } }
pub extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int { unsafe { musl_fgetc_unlocked(f) } }
pub extern "C" fn getc_unlocked(f: *mut FILE) -> c_int { unsafe { musl_getc_unlocked(f) } }
pub extern "C" fn getchar_unlocked() -> c_int { unsafe { musl_getchar_unlocked() } }
pub extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_fputc_unlocked(c, f) } }
pub extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_putc_unlocked(c, f) } }
pub extern "C" fn putchar_unlocked(c: c_int) -> c_int { unsafe { musl_putchar_unlocked(c) } }
pub extern "C" fn feof_unlocked(f: *mut FILE) -> c_int { unsafe { musl_feof_unlocked(f) } }
pub extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int { unsafe { musl_ferror_unlocked(f) } }
pub extern "C" fn clearerr_unlocked(f: *mut FILE) { unsafe { musl_clearerr_unlocked(f) } }
pub extern "C" fn fileno_unlocked(f: *mut FILE) -> c_int { unsafe { musl_fileno_unlocked(f) } }

// ---------------------------------------------------------------------------
// 线程安全 / 锁定
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "flockfile"]
    fn musl_flockfile(f: *mut FILE);
    #[link_name = "ftrylockfile"]
    fn musl_ftrylockfile(f: *mut FILE) -> c_int;
    #[link_name = "funlockfile"]
    fn musl_funlockfile(f: *mut FILE);
}

pub extern "C" fn flockfile(f: *mut FILE) { unsafe { musl_flockfile(f) } }
pub extern "C" fn ftrylockfile(f: *mut FILE) -> c_int { unsafe { musl_ftrylockfile(f) } }
pub extern "C" fn funlockfile(f: *mut FILE) { unsafe { musl_funlockfile(f) } }

// ---------------------------------------------------------------------------
// 宽字符 I/O
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fgetwc"]
    fn musl_fgetwc(f: *mut FILE) -> c_int;
    #[link_name = "fputwc"]
    fn musl_fputwc(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "getwc"]
    fn musl_getwc(f: *mut FILE) -> c_int;
    #[link_name = "putwc"]
    fn musl_putwc(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "getwchar"]
    fn musl_getwchar() -> c_int;
    #[link_name = "putwchar"]
    fn musl_putwchar(c: c_int) -> c_int;
    #[link_name = "ungetwc"]
    fn musl_ungetwc(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "fgetws"]
    fn musl_fgetws(ws: *mut c_int, n: c_int, f: *mut FILE) -> *mut c_int;
    #[link_name = "fputws"]
    fn musl_fputws(ws: *const c_int, f: *mut FILE) -> c_int;
}

pub extern "C" fn fgetwc(f: *mut FILE) -> c_int { unsafe { musl_fgetwc(f) } }
pub extern "C" fn fputwc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_fputwc(c, f) } }
pub extern "C" fn getwc(f: *mut FILE) -> c_int { unsafe { musl_getwc(f) } }
pub extern "C" fn putwc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_putwc(c, f) } }
pub extern "C" fn getwchar() -> c_int { unsafe { musl_getwchar() } }
pub extern "C" fn putwchar(c: c_int) -> c_int { unsafe { musl_putwchar(c) } }
pub extern "C" fn ungetwc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_ungetwc(c, f) } }
pub extern "C" fn fgetws(ws: *mut c_int, n: c_int, f: *mut FILE) -> *mut c_int {
    unsafe { musl_fgetws(ws, n, f) }
}
pub extern "C" fn fputws(ws: *const c_int, f: *mut FILE) -> c_int { unsafe { musl_fputws(ws, f) } }

// ---------------------------------------------------------------------------
// 宽字符免锁变体
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fgetwc_unlocked"]
    fn musl_fgetwc_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "getwc_unlocked"]
    fn musl_getwc_unlocked(f: *mut FILE) -> c_int;
    #[link_name = "getwchar_unlocked"]
    fn musl_getwchar_unlocked() -> c_int;
    #[link_name = "fputwc_unlocked"]
    fn musl_fputwc_unlocked(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "putwc_unlocked"]
    fn musl_putwc_unlocked(c: c_int, f: *mut FILE) -> c_int;
    #[link_name = "putwchar_unlocked"]
    fn musl_putwchar_unlocked(c: c_int) -> c_int;
    #[link_name = "fgetws_unlocked"]
    fn musl_fgetws_unlocked(ws: *mut c_int, n: c_int, f: *mut FILE) -> *mut c_int;
    #[link_name = "fputws_unlocked"]
    fn musl_fputws_unlocked(ws: *const c_int, f: *mut FILE) -> c_int;
}

pub extern "C" fn fgetwc_unlocked(f: *mut FILE) -> c_int { unsafe { musl_fgetwc_unlocked(f) } }
pub extern "C" fn getwc_unlocked(f: *mut FILE) -> c_int { unsafe { musl_getwc_unlocked(f) } }
pub extern "C" fn getwchar_unlocked() -> c_int { unsafe { musl_getwchar_unlocked() } }
pub extern "C" fn fputwc_unlocked(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_fputwc_unlocked(c, f) } }
pub extern "C" fn putwc_unlocked(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_putwc_unlocked(c, f) } }
pub extern "C" fn putwchar_unlocked(c: c_int) -> c_int { unsafe { musl_putwchar_unlocked(c) } }
pub extern "C" fn fgetws_unlocked(ws: *mut c_int, n: c_int, f: *mut FILE) -> *mut c_int {
    unsafe { musl_fgetws_unlocked(ws, n, f) }
}
pub extern "C" fn fputws_unlocked(ws: *const c_int, f: *mut FILE) -> c_int {
    unsafe { musl_fputws_unlocked(ws, f) }
}

// ---------------------------------------------------------------------------
// 宽字符格式化输出 — va_list 版本
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "vfwprintf"]
    fn musl_vfwprintf(f: *mut FILE, fmt: *const c_int, ap: VaList) -> c_int;
    #[link_name = "vwprintf"]
    fn musl_vwprintf(fmt: *const c_int, ap: VaList) -> c_int;
    #[link_name = "vswprintf"]
    fn musl_vswprintf(s: *mut c_int, n: usize, fmt: *const c_int, ap: VaList) -> c_int;
}

pub extern "C" fn vfwprintf(f: *mut FILE, fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vfwprintf(f, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vwprintf(fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vwprintf(fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vswprintf(s: *mut c_int, n: usize, fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vswprintf(s, n, fmt, core::ptr::read(ap)) }
}

// ---------------------------------------------------------------------------
// 宽字符格式化输出 — 可变参数版本
// ---------------------------------------------------------------------------

extern "C" {
    pub fn wprintf(fmt: *const c_int, ...) -> c_int;
    pub fn fwprintf(f: *mut FILE, fmt: *const c_int, ...) -> c_int;
    pub fn swprintf(s: *mut c_int, n: usize, fmt: *const c_int, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// 宽字符格式化输入 — va_list 版本
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "vfwscanf"]
    fn musl_vfwscanf(f: *mut FILE, fmt: *const c_int, ap: VaList) -> c_int;
    #[link_name = "vwscanf"]
    fn musl_vwscanf(fmt: *const c_int, ap: VaList) -> c_int;
    #[link_name = "vswscanf"]
    fn musl_vswscanf(s: *const c_int, fmt: *const c_int, ap: VaList) -> c_int;
}

pub extern "C" fn vfwscanf(f: *mut FILE, fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vfwscanf(f, fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vwscanf(fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vwscanf(fmt, core::ptr::read(ap)) }
}
pub extern "C" fn vswscanf(s: *const c_int, fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe { musl_vswscanf(s, fmt, core::ptr::read(ap)) }
}

// ---------------------------------------------------------------------------
// 宽字符格式化输入 — 可变参数版本
// ---------------------------------------------------------------------------

extern "C" {
    pub fn wscanf(fmt: *const c_int, ...) -> c_int;
    pub fn fwscanf(f: *mut FILE, fmt: *const c_int, ...) -> c_int;
    pub fn swscanf(s: *const c_int, fmt: *const c_int, ...) -> c_int;
}

// ---------------------------------------------------------------------------
// GNU 扩展
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "fopencookie"]
    fn musl_fopencookie(
        cookie: *mut c_void,
        mode: *const c_char,
        io_funcs: cookie_io_functions_t,
    ) -> *mut FILE;
    #[link_name = "fpurge"]
    fn musl_fpurge(f: *mut FILE) -> c_int;
}

pub extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    io_funcs: cookie_io_functions_t,
) -> *mut FILE {
    unsafe { musl_fopencookie(cookie, mode, io_funcs) }
}
pub extern "C" fn fpurge(f: *mut FILE) -> c_int { unsafe { musl_fpurge(f) } }
