//! flockfile — 文件流阻塞锁定。
//! 对应 musl src/stdio/flockfile.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// flockfile — 获取文件流的递归锁，若不能立即获取则阻塞等待。
/// 先尝试 ftrylockfile 非阻塞获取，失败则降级为 __lockfile 阻塞等待。
#[no_mangle]
pub extern "C" fn flockfile(_f: *mut FILE) {
    unimplemented!()
}

/// ftrylockfile — 非阻塞尝试获取文件流锁。
/// 成功返回 0，失败返回 -1。
#[no_mangle]
pub extern "C" fn ftrylockfile(_f: *mut FILE) -> c_int {
    unimplemented!()
}

/// funlockfile — 释放文件流锁。
#[no_mangle]
pub extern "C" fn funlockfile(_f: *mut FILE) {
    unimplemented!()
}
