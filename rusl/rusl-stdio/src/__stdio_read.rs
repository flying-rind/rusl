//! 对应 musl src/stdio/__stdio_read.c
//! 内部 FILE 默认读操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 从文件描述符读取数据到用户缓冲区
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    unimplemented!()
}
