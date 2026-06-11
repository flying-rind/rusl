//! getline — 以换行符为分隔的动态行读取。
//! 对应 musl src/stdio/getline.c
//! 等价于 getdelim(s, n, '\n', f)

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取以 '\n' 结尾的一行数据到动态分配的缓冲区 *s。
/// [Visibility]: User — POSIX.1-2008 标准函数。
#[no_mangle]
pub extern "C" fn getline(
    s: *mut *mut c_char,
    n: *mut usize,
    f: *mut FILE,
) -> isize {
    unimplemented!()
}
