//! C 兼容的 FILE 类型定义。
//!
//! 对应 musl `src/internal/stdio_impl.h` 中的 `struct _IO_FILE`。
//! 字节布局必须与 C 定义完全一致。

use core::ffi::{c_int, c_long, c_uint, c_void};

/// C ABI 兼容的 FILE 结构体。
///
/// 对应 musl `typedef struct _IO_FILE FILE;`
/// 在 x86_64 上总大小为 232 字节。
#[repr(C)]
pub struct FILE {
    pub flags: c_uint,
    _pad0: [u8; 4],
    pub rpos: *mut u8,
    pub rend: *mut u8,
    pub close: *mut c_void,
    pub wend: *mut u8,
    pub wpos: *mut u8,
    pub mustbezero_1: *mut u8,
    pub wbase: *mut u8,
    pub read: *mut c_void,
    pub write: *mut c_void,
    pub seek: *mut c_void,
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
    _pad1: [u8; 4],
    pub cookie: *mut c_void,
    pub off: i64,
    pub getln_buf: *mut u8,
    pub mustbezero_2: *mut c_void,
    pub shend: *mut u8,
    pub shlim: i64,
    pub shcnt: i64,
    pub prev_locked: *mut FILE,
    pub next_locked: *mut FILE,
    pub locale: *mut c_void,
}
