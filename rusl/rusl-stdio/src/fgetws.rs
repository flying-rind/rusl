//! fgetws — 从 FILE 流中读取一行宽字符串。
//! 对应 musl src/stdio/fgetws.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fgetws — 从 FILE 流中读取至多 n-1 个宽字符到 s，遇到 L'\n' 或 EOF 时停止。
/// 读取后以 L'\0' 终止。返回 s（成功）或 NULL（失败且未读取任何字符）。
#[no_mangle]
pub extern "C" fn fgetws(
    _s: *mut core::ffi::c_uint,
    _n: c_int,
    _f: *mut FILE,
) -> *mut core::ffi::c_uint {
    unimplemented!()
}

/// fgetws_unlocked — fgetws 的弱别名。行为与 fgetws 完全一致。
#[no_mangle]
pub extern "C" fn fgetws_unlocked(
    _s: *mut core::ffi::c_uint,
    _n: c_int,
    _f: *mut FILE,
) -> *mut core::ffi::c_uint {
    unimplemented!()
}
