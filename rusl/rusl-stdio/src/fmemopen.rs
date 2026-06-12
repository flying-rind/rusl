//! fmemopen — 创建内存流 FILE 对象。
//! 对应 musl src/stdio/fmemopen.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_void};

/// fmemopen — 创建对内存缓冲区进行 I/O 的 FILE 流。
#[no_mangle]
pub extern "C" fn fmemopen(
    _buf: *mut c_void,
    _size: usize,
    _mode: *const c_char,
) -> *mut FILE {
    core::ptr::null_mut()
}
