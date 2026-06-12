//! 对应 musl src/stdio/setbuffer.c
//! GNU 扩展，为 FILE 流设置缓冲模式和自定义大小缓冲区

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

const _IOFBF: i32 = 0;
const _IONBF: i32 = 2;

/// 设置流缓冲：buf 非 null 时全缓冲 size，null 时无缓冲
#[no_mangle]
pub extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: usize) {
    super::setvbuf::setvbuf(f, buf, if buf.is_null() { _IONBF } else { _IOFBF }, size);
}
