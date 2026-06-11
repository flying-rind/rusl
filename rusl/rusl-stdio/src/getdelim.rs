//! getdelim — 带分隔符的动态行读取。
//! 对应 musl src/stdio/getdelim.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取以字符 delim 分隔（或 EOF 结尾）的一行数据到动态分配的缓冲区 *s。
/// 返回读取的字符数（含分隔符），-1 表示错误。
/// [Visibility]: User — POSIX.1-2008 标准函数。
#[no_mangle]
pub extern "C" fn getdelim(
    s: *mut *mut c_char,
    n: *mut usize,
    delim: core::ffi::c_int,
    f: *mut FILE,
) -> isize {
    unimplemented!()
}

/// 内部别名（弱别名 -> getdelim），供 musl 内部直接调用。
/// [Visibility]: Internal。
#[no_mangle]
pub(crate) unsafe extern "C" fn __getdelim(
    s: *mut *mut c_char,
    n: *mut usize,
    delim: core::ffi::c_int,
    f: *mut FILE,
) -> isize {
    unimplemented!()
}
