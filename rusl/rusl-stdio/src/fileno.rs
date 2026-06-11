//! fileno — 获取文件流底层文件描述符。
//! 对应 musl src/stdio/fileno.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fileno — 获取与 FILE 流关联的底层文件描述符（加锁版本）。
/// 返回非负文件描述符（成功），或 -1 且 errno = EBADF（流未关联有效 fd）。
#[no_mangle]
pub extern "C" fn fileno(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// fileno_unlocked — fileno 的弱别名。行为与 fileno 完全一致。
#[no_mangle]
pub extern "C" fn fileno_unlocked(_f: *mut FILE) -> c_int {
    unimplemented!()
}
