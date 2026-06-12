//! 对应 musl src/stdio/__stdio_seek.c
//! 内部 FILE 默认定位操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_lseek: i64 = 8;
#[cfg(target_arch = "aarch64")]
const SYS_lseek: i64 = 62;

/// 将定位请求直接转发给 lseek 系统调用
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let f_ref = &*f;
    rusl_core::__syscall3(SYS_lseek, f_ref.fd as i64, off, whence as i64)
}
