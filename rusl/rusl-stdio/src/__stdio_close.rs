//! 对应 musl src/stdio/__stdio_close.c
//! 内部 FILE 默认关闭操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_close: i64 = 3;
#[cfg(target_arch = "aarch64")]
const SYS_close: i64 = 57;

/// 关闭 FILE 关联的文件描述符
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_close(f: *mut FILE) -> c_int {
    let f_ref = &*f;
    let ret = rusl_core::__syscall1(SYS_close, f_ref.fd as i64);
    if ret < 0 { super::stdio_impl::EOF } else { 0 }
}
