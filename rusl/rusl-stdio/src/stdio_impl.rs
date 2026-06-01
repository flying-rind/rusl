//! stdio 内部实现 —— FILE 结构体、常量、va_list 定义。
//! 对应 musl src/internal/stdio_impl.h

use core::ffi::{c_char, c_int, c_long, c_uint, c_void};

// ---------------------------------------------------------------------------
// F_* 标志位（musl: #define F_PERM 1, F_NORD 4, ...）
// ---------------------------------------------------------------------------
pub const F_PERM: c_uint = 1;
pub const F_NORD: c_uint = 4;
pub const F_NOWR: c_uint = 8;
pub const F_EOF: c_uint = 16;
pub const F_ERR: c_uint = 32;
pub const F_SVB: c_uint = 64;
pub const F_APP: c_uint = 128;

// ---------------------------------------------------------------------------
// EOF / UNGET / 错误码
// ---------------------------------------------------------------------------
pub const EOF: c_int = -1;
pub const UNGET: usize = 8;

// ---------------------------------------------------------------------------
// NL_ARGMAX (musl: <limits.h>)
// ---------------------------------------------------------------------------
pub const NL_ARGMAX: usize = 9;

// ---------------------------------------------------------------------------
// INT_MAX（从 core 获取）
// ---------------------------------------------------------------------------
pub const INT_MAX: c_int = c_int::MAX;
pub const ULONG_MAX: u64 = u64::MAX;

// ---------------------------------------------------------------------------
// FILE 结构体（musl: struct _IO_FILE）
// 必须在布局上与 C 侧完全一致。
// ---------------------------------------------------------------------------
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

/// 便捷 inline：ferror 谓词。
#[inline]
pub unsafe fn ferror(f: *const FILE) -> bool {
    (*f).flags & F_ERR != 0
}

// ---------------------------------------------------------------------------
// va_list (x86_64 System V AMD64 ABI)
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct VaList {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

/// x86_64 va_arg 提取整数/指针类型参数。
/// 调用者负责保证类型安全。
#[inline]
pub unsafe fn va_arg_int(ap: *mut VaList) -> i32 {
    let ap = &mut *ap;
    if ap.gp_offset + 4 <= 48 {
        // 仍在寄存器保存区
        let ptr = (ap.reg_save_area as *const u8).add(ap.gp_offset as usize);
        ap.gp_offset += 8;
        (ptr as *const i32).read_unaligned()
    } else {
        // 从溢出区取
        let ptr = ap.overflow_arg_area;
        // 对齐到 8 后再前进
        ap.overflow_arg_area =
            ((ap.overflow_arg_area as usize + 7) & !7) as *mut c_void;
        let val = (ptr as *const i32).read_unaligned();
        ap.overflow_arg_area = (ap.overflow_arg_area as *mut u8).add(8) as *mut c_void;
        val
    }
}

#[inline]
pub unsafe fn va_arg_uint(ap: *mut VaList) -> u32 {
    va_arg_int(ap) as u32
}

#[inline]
pub unsafe fn va_arg_long(ap: *mut VaList) -> i64 {
    let ap = &mut *ap;
    if ap.gp_offset + 8 <= 48 {
        let ptr = (ap.reg_save_area as *const u8).add(ap.gp_offset as usize);
        ap.gp_offset += 8;
        (ptr as *const i64).read_unaligned()
    } else {
        let ptr = ap.overflow_arg_area;
        ap.overflow_arg_area =
            ((ap.overflow_arg_area as usize + 7) & !7) as *mut c_void;
        let val = (ptr as *const i64).read_unaligned();
        ap.overflow_arg_area = (ap.overflow_arg_area as *mut u8).add(8) as *mut c_void;
        val
    }
}

#[inline]
pub unsafe fn va_arg_ulong(ap: *mut VaList) -> u64 {
    va_arg_long(ap) as u64
}

#[inline]
pub unsafe fn va_arg_ulonglong(ap: *mut VaList) -> u64 {
    va_arg_long(ap) as u64
}

#[inline]
pub unsafe fn va_arg_longlong(ap: *mut VaList) -> i64 {
    va_arg_long(ap)
}

#[inline]
pub unsafe fn va_arg_ptr(ap: *mut VaList) -> *mut c_void {
    let ap = &mut *ap;
    if ap.gp_offset + 8 <= 48 {
        let ptr = (ap.reg_save_area as *const u8).add(ap.gp_offset as usize);
        ap.gp_offset += 8;
        (ptr as *const usize).read_unaligned() as *mut c_void
    } else {
        let ptr = ap.overflow_arg_area;
        ap.overflow_arg_area =
            ((ap.overflow_arg_area as usize + 7) & !7) as *mut c_void;
        let val = (ptr as *const usize).read_unaligned() as *mut c_void;
        ap.overflow_arg_area = (ap.overflow_arg_area as *mut u8).add(8) as *mut c_void;
        val
    }
}
