//! wprintf — 宽字符标准输出格式化。
//! 对应 musl src/stdio/wprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// wprintf — 将格式化宽字符串输出到 stdout。
#[no_mangle]
pub unsafe extern "C" fn wprintf(fmt: *const c_int, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut VaList;
    let f = unsafe { super::stdout::stdout };
    super::vfwprintf::vfwprintf(f, fmt, ap)
}
