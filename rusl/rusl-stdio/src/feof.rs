//! feof — 文件流 EOF 状态查询。
//! 对应 musl src/stdio/feof.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// feof — 测试文件流的文件结束指示符（加锁版本）。
/// 返回 0（EOF 未到达）或 1（EOF 已到达）。
#[no_mangle]
pub extern "C" fn feof(f: *mut FILE) -> c_int {
    unsafe {
        let f_ref = &*f;
        if (f_ref.flags & F_EOF) != 0 { 1 } else { 0 }
    }
}

/// feof_unlocked — feof 的弱别名。行为与 feof 完全一致。
#[no_mangle]
pub extern "C" fn feof_unlocked(f: *mut FILE) -> c_int {
    feof(f)
}

/// _IO_feof_unlocked — feof 的弱别名。glibc 兼容符号。
#[no_mangle]
pub(crate) unsafe extern "C" fn _IO_feof_unlocked(f: *mut FILE) -> c_int {
    feof(f)
}
