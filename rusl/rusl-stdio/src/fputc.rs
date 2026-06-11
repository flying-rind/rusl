//! fputc — 将单个字符写入 FILE 流。
//! 对应 musl src/stdio/fputc.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use crate::stdio_impl::FILE;

/// 将字符 c（转换为 unsigned char）写入 FILE 流 f。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputc(c: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}
