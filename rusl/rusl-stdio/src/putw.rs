//! 对应 musl src/stdio/putw.c
//! 整数二进制写入实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 将 c_int 值 x 的二进制表示写入 FILE 流 f
#[no_mangle]
pub extern "C" fn putw(x: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}
