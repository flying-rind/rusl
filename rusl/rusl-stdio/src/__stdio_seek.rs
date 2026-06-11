//! 对应 musl src/stdio/__stdio_seek.c
//! 内部 FILE 默认定位操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 将定位请求直接转发给 __lseek 系统调用
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    unimplemented!()
}
