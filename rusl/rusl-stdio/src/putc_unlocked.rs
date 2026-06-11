//! 对应 musl src/stdio/putc_unlocked.c
//! 免锁 FILE 流单字符写入

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 将字符 c 写入 FILE 流 f，不获取流锁
#[no_mangle]
pub extern "C" fn putc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}

/// fputc_unlocked — putc_unlocked 的弱别名，POSIX 标准名称
#[no_mangle]
pub extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}

/// _IO_putc_unlocked — putc_unlocked 的弱别名，glibc 兼容符号
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int {
    unimplemented!()
}
