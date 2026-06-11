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
}

pub extern "C" fn fgetc(f: *mut FILE) -> c_int { unsafe { musl_fgetc(f) } }
pub extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_fputc(c, f) } }
pub extern "C" fn getc(f: *mut FILE) -> c_int { unsafe { musl_getc(f) } }
pub extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int { unsafe { musl_putc(c, f) } }
pub extern "C" fn getchar() -> c_int { unsafe { musl_getchar() } }
pub extern "C" fn putchar(c: c_int) -> c_int { unsafe { musl_putchar(c) } }
pub extern "C" fn puts(s: *const c_char) -> c_int { unsafe { musl_puts(s) } }

// ---------------------------------------------------------------------------
// 格式化 — va_list 版本（vfprintf / vsnprintf / vsprintf）
// 统一使用 *mut VaList, 与 rusl_stdio 接口一致。
// 内部通过解引用转为按值传递以匹配 musl C ABI。
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
// Rust 无法定义 extern "C" 可变参数函数体,
// 因此仅声明符号供 FFI 调用, 由 musl libc 提供实际实现。
// ---------------------------------------------------------------------------

extern "C" {
    // 可变参数函数 — 只能声明, 不能定义 Rust 包装体。
    // 测试代码直接调用这些 extern 符号。
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
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

// ---------------------------------------------------------------------------
// 缓冲控制
// ---------------------------------------------------------------------------

extern "C" {
    #[link_name = "setbuf"]
    fn musl_setbuf(f: *mut FILE, buf: *mut c_char);
    #[link_name = "setvbuf"]
    fn musl_setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
}

pub extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char) {
    unsafe { musl_setbuf(f, buf) }
}
pub extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int {
    unsafe { musl_setvbuf(f, buf, mode, size) }
}
