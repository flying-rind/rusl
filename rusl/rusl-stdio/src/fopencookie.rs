//! fopencookie — 创建用户自定义回调驱动的 FILE 流。
//! 对应 musl src/stdio/fopencookie.c
//! GNU 扩展接口（需 _GNU_SOURCE）。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use super::stdio_impl::FILE;

/// cookie_io_functions_t: 用户提供的 I/O 回调函数集合。
#[repr(C)]
pub struct cookie_io_functions_t {
    pub read: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    pub write: Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>,
    pub seek: Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>,
    pub close: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

/// 创建用户自定义回调流。
#[no_mangle]
pub extern "C" fn fopencookie(
    _cookie: *mut c_void,
    _mode: *const c_char,
    _iofuncs: cookie_io_functions_t,
) -> *mut FILE {
    core::ptr::null_mut()
}
