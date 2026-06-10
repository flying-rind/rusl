//! __uflow — 从 FILE 流读取一个字符。
//! 对应 musl src/stdio/__uflow.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 从 FILE 流获取下一个字符（仅在缓冲区为空时调用）。
#[no_mangle]
pub unsafe extern "C" fn __uflow(f: *mut FILE) -> c_int {
    if super::__toread::__toread(f) != 0 {
        return EOF;
    }
    if let Some(read_fn) = (*f).read {
        let mut c: u8 = 0;
        if read_fn(f, &raw mut c, 1) == 1 {
            return c as c_int;
        }
    }
    EOF
}
