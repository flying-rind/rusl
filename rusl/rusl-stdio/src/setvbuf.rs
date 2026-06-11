//! 对应 musl src/stdio/setvbuf.c
//! 所有缓冲设置函数的最终实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// 设置 FILE 流的缓冲模式、缓冲区位置和大小
#[no_mangle]
pub extern "C" fn setvbuf(
    f: *mut FILE,
    buf: *mut c_char,
    type_: c_int,
    size: usize,
) -> c_int {
    unimplemented!()
}
