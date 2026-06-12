//! tempnam — 可定制临时文件名生成（POSIX XSI 扩展，已过时）。
//! 对应 musl src/stdio/tempnam.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// tempnam — 生成唯一临时文件名。
#[no_mangle]
pub extern "C" fn tempnam(_dir: *const c_char, _pfx: *const c_char) -> *mut c_char {
    core::ptr::null_mut()
}
