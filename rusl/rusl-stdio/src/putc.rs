//! 对应 musl src/stdio/putc.c
//! 标准 IO 宏兼容字符写入实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;
use super::putc_unlocked::putc_unlocked;

/// 将字符 c 写入 FILE 流 f
#[no_mangle]
pub extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int {
    unsafe { putc_unlocked(c, f) }
}

/// _IO_putc — putc 的弱别名
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_putc(c: c_int, f: *mut FILE) -> c_int {
    unsafe { putc_unlocked(c, f) }
}
