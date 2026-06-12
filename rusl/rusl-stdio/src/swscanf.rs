//! swscanf — 从宽字符串读取格式化输入。
//! 对应 musl src/stdio/swscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// swscanf — 从宽字符串 s 读取格式化输入。
#[no_mangle]
pub unsafe extern "C" fn swscanf(s: *const c_int, fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    super::vswscanf::vswscanf(s, fmt, ap)
}

/// __isoc99_swscanf — swscanf 的 C99 兼容弱别名。
#[no_mangle]
pub unsafe extern "C" fn __isoc99_swscanf(s: *const c_int, fmt: *const c_int, args: ...) -> c_int {
    swscanf(s, fmt, args)
}
