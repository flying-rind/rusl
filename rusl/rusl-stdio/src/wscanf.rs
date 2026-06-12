//! wscanf — 宽字符标准输入格式化读取。
//! 对应 musl src/stdio/wscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// wscanf — 从 stdin 读取宽字符格式化输入。
#[no_mangle]
pub unsafe extern "C" fn wscanf(fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    let f = unsafe { super::stdin::stdin };
    super::vfwscanf::vfwscanf(f, fmt, ap)
}

/// __isoc99_wscanf — wscanf 的 C99 兼容弱别名。
#[no_mangle]
pub unsafe extern "C" fn __isoc99_wscanf(fmt: *const c_int, args: ...) -> c_int {
    wscanf(fmt, args)
}
