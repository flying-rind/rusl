//! getc — 从 FILE 流读取一个字符（宏的函数级回退实现）。
//! 对应 musl src/stdio/getc.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取一个字符（加锁）。
/// [Visibility]: User — <stdio.h> 标准库函数（宏的备选函数实现）。
#[no_mangle]
pub extern "C" fn getc(f: *mut FILE) -> c_int {
    unimplemented!()
}

/// glibc 兼容别名（弱别名 -> getc）。
/// [Visibility]: Internal — 供 libstdc++ 等传统 _IO_ 前缀代码使用。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_getc(f: *mut FILE) -> c_int {
    unimplemented!()
}
