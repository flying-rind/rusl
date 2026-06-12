//! getline — 以换行符为分隔的动态行读取。
//! 对应 musl src/stdio/getline.c
//! 等价于 getdelim(s, n, '\n', f)

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// getline — 获取一行（'\n' 为分隔符），等价于 getdelim(lineptr, n, '\n', f)。
#[no_mangle]
pub extern "C" fn getline(
    _lineptr: *mut *mut c_char,
    _n: *mut usize,
    _f: *mut FILE,
) -> isize {
    -1
}
