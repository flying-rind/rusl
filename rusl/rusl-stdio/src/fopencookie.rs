//! fopencookie — 创建用户自定义回调驱动的 FILE 流。
//! 对应 musl src/stdio/fopencookie.c
//! GNU 扩展接口（需 _GNU_SOURCE）。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int, c_void};
use crate::stdio_impl::FILE;

/// cookie_io_functions_t: 用户提供的 I/O 回调函数集合。
/// 对应 C 的 cookie_io_functions_t，定义于 <stdio.h>
#[repr(C)]
pub struct cookie_io_functions_t {
    /// 读取回调: (cookie, buf, len) -> 实际读取字节数或负数表示错误
    pub read: Option<unsafe extern "C" fn(*mut c_void, *mut c_char, usize) -> isize>,
    /// 写入回调: (cookie, buf, len) -> 实际写入字节数或负数表示错误
    pub write: Option<unsafe extern "C" fn(*mut c_void, *const c_char, usize) -> isize>,
    /// seek 回调: (cookie, *offset, whence) -> 0 成功, -1 失败
    pub seek: Option<unsafe extern "C" fn(*mut c_void, *mut i64, c_int) -> c_int>,
    /// 关闭回调: (cookie) -> 0 成功, -1 失败
    pub close: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
}

/// 创建用户自定义回调流。
/// [Visibility]: User — GNU 扩展函数（需 _GNU_SOURCE）。
#[no_mangle]
pub extern "C" fn fopencookie(
    cookie: *mut c_void,
    mode: *const c_char,
    iofuncs: cookie_io_functions_t,
) -> *mut FILE {
    unimplemented!()
}
