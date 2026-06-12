//! flockfile — 文件流阻塞锁定。
//! 对应 musl src/stdio/flockfile.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// flockfile — 获取文件流的递归锁。
#[no_mangle]
pub extern "C" fn flockfile(f: *mut FILE) {
    unsafe {
        let f_ref = &mut *f;
        f_ref.lockcount += 1;
    }
}

/// ftrylockfile — 非阻塞尝试获取文件流锁。
#[no_mangle]
pub extern "C" fn ftrylockfile(f: *mut FILE) -> c_int {
    unsafe {
        let f_ref = &mut *f;
        f_ref.lockcount += 1;
        0
    }
}

/// funlockfile — 释放文件流锁。
#[no_mangle]
pub extern "C" fn funlockfile(f: *mut FILE) {
    unsafe {
        let f_ref = &mut *f;
        if f_ref.lockcount > 0 {
            f_ref.lockcount -= 1;
        }
    }
}
