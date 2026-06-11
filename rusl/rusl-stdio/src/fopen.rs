//! fopen / freopen — 文件打开与重定向。
//! 对应 musl src/stdio/fopen.c 和 src/stdio/freopen.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use crate::stdio_impl::FILE;

/// 根据文件名和模式打开文件，返回缓冲的 FILE 流。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fopen(
    filename: *const c_char,
    mode: *const c_char,
) -> *mut FILE {
    unimplemented!()
}

/// 将已有 FILE 流重定向到新文件路径，或修改当前 fd 的模式。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn freopen(
    filename: *const c_char,
    mode: *const c_char,
    f: *mut FILE,
) -> *mut FILE {
    unimplemented!()
}
