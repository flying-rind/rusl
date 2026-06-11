//! 对应 musl src/stdio/setbuf.c
//! 为 FILE 流设置缓冲模式和缓冲区，setvbuf 的简化包装

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// 设置流缓冲：buf 非 null 时全缓冲 BUFSIZ，null 时无缓冲
#[no_mangle]
pub extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char) {
    unimplemented!()
}
