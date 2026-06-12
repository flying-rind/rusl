//! getdelim — 带分隔符的动态行读取。
//! 对应 musl src/stdio/getdelim.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// getdelim — 从 FILE 流读取行数据到动态分配的缓冲区。
#[no_mangle]
pub extern "C" fn getdelim(
    _lineptr: *mut *mut c_char,
    _n: *mut usize,
    _delim: c_int,
    _f: *mut FILE,
) -> isize {
    -1
}
