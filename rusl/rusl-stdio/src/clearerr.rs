//! clearerr — 清除文件流错误状态。
//! 对应 musl src/stdio/clearerr.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// clearerr — 清除 FILE 流的 EOF 和 ERR 标志位（加锁版本）。
#[no_mangle]
pub extern "C" fn clearerr(_f: *mut FILE) {
    unimplemented!()
}

/// clearerr_unlocked — clearerr 的 POSIX 免锁扩展（弱别名，musl 中与 clearerr 同实现）。
#[no_mangle]
pub extern "C" fn clearerr_unlocked(_f: *mut FILE) {
    unimplemented!()
}
