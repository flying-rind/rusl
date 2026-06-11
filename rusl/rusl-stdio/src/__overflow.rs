//! 对应 musl src/stdio/__overflow.c
//! 内部输出缓冲区溢出处理实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 处理 stdio 输出缓冲区的"溢出"情况
#[no_mangle]
pub(crate) unsafe extern "C" fn __overflow(f: *mut FILE, _c: c_int) -> c_int {
    unimplemented!()
}
