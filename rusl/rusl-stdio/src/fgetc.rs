//! fgetc — 从 FILE 流读取单个字符。
//! 对应 musl src/stdio/fgetc.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fgetc — 从 FILE 流中读取一个 unsigned char 字符，以 int 返回。
/// 到达 EOF 或发生错误时返回 EOF。
#[no_mangle]
pub extern "C" fn fgetc(_f: *mut FILE) -> c_int {
    unimplemented!()
}
