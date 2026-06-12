//! fwscanf — 从 FILE 流宽字符格式化输入。
//! 对应 musl src/stdio/fwscanf.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;

/// fwscanf — 宽字符格式化输入（可变参数版本）。
#[no_mangle]
pub unsafe extern "C" fn fwscanf(f: *mut FILE, fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut super::stdio_impl::VaList;
    super::vfwscanf::vfwscanf(f, fmt, ap)
}
