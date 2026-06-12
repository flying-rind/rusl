//! fwprintf — 宽字符格式化输出到 FILE 流。
//! 对应 musl src/stdio/fwprintf.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_char};
use super::stdio_impl::FILE;

/// fwprintf — 宽字符格式化输出（可变参数版本）。
#[no_mangle]
pub unsafe extern "C" fn fwprintf(f: *mut FILE, fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut super::stdio_impl::VaList;
    super::vfwprintf::vfwprintf(f, fmt, ap)
}
