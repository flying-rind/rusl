//! ferror — 文件流错误状态查询。
//! 对应 musl src/stdio/ferror.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// ferror — 测试文件流的错误指示符（加锁版本）。
/// 返回 0（无错误）或 1（有错误）。
#[no_mangle]
pub extern "C" fn ferror(f: *mut FILE) -> c_int {
    unsafe {
        let f_ref = &*f;
        if (f_ref.flags & F_ERR) != 0 { 1 } else { 0 }
    }
}

/// ferror_unlocked — ferror 的弱别名。行为与 ferror 完全一致。
#[no_mangle]
pub extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int {
    ferror(f)
}

/// _IO_ferror_unlocked — ferror 的弱别名。glibc 兼容符号。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_ferror_unlocked(f: *mut FILE) -> c_int {
    ferror(f)
}
