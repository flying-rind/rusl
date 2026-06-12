//! tmpnam — 临时文件名生成（C89，已过时）。
//! 对应 musl src/stdio/tmpnam.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// tmpnam — 生成唯一临时文件名。
#[no_mangle]
pub extern "C" fn tmpnam(_buf: *mut c_char) -> *mut c_char {
    core::ptr::null_mut()
}
