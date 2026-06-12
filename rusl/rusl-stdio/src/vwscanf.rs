//! vwscanf — 宽字符标准输入格式化读取（va_list 版本）。
//! 对应 musl src/stdio/vwscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vwscanf — 从 stdin 读取宽字符格式化输入。
#[no_mangle]
pub extern "C" fn vwscanf(fmt: *const c_int, ap: *mut VaList) -> c_int {
    let f = unsafe { super::stdin::stdin };
    super::vfwscanf::vfwscanf(f, fmt, ap)
}

/// __isoc99_vwscanf — vwscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vwscanf(fmt: *const c_int, ap: *mut VaList) -> c_int {
    vwscanf(fmt, ap)
}
