//! 对应 musl src/stdio/putwc.c
//! 宽字符输出函数，等价于 fputwc(c, f)

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 将宽字符 c 写入 FILE 流 f，等价于 fputwc(c, f)
#[no_mangle]
pub extern "C" fn putwc(c: c_int, f: *mut FILE) -> c_int {
    super::fputwc::fputwc(c, f)
}
