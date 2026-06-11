//! 对应 musl src/stdio/__stdio_close.c
//! 内部 FILE 默认关闭操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 关闭 FILE 关联的文件描述符
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_close(f: *mut FILE) -> c_int {
    unimplemented!()
}
