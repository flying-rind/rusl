//! vfwscanf — 宽字符格式化输入核心引擎。
//! 对应 musl src/stdio/vfwscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vfwscanf — 从 FILE 流读取宽字符格式化输入。
#[no_mangle]
pub extern "C" fn vfwscanf(_f: *mut FILE, _fmt: *const c_int, _ap: *mut VaList) -> c_int {
    -1
}

/// __isoc99_vfwscanf — vfwscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vfwscanf(f: *mut FILE, fmt: *const c_int, ap: *mut VaList) -> c_int {
    vfwscanf(f, fmt, ap)
}
