//! vwprintf — 宽字符标准输出格式化（va_list 版本）。
//! 对应 musl src/stdio/vwprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// vwprintf — 将格式化宽字符串输出到 stdout。
#[no_mangle]
pub extern "C" fn vwprintf(fmt: *const c_int, ap: *mut VaList) -> c_int {
    let f = unsafe { super::stdout::stdout };
    super::vfwprintf::vfwprintf(f, fmt, ap)
}
