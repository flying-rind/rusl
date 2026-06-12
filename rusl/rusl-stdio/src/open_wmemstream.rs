//! 对应 musl src/stdio/open_wmemstream.c
//! 创建宽字符动态内存流

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 创建宽字符动态内存流。
#[no_mangle]
pub extern "C" fn open_wmemstream(
    _bufp: *mut *mut c_int,
    _sizep: *mut usize,
) -> *mut FILE {
    core::ptr::null_mut()
}
