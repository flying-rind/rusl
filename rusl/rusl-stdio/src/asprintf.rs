//! asprintf — 自动分配缓冲区的格式化输出。
//! 对应 musl src/stdio/asprintf.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// asprintf — 自动分配缓冲区并格式化输出（可变参数版本）。
#[no_mangle]
pub unsafe extern "C" fn asprintf(s: *mut *mut c_char, fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut super::stdio_impl::VaList;
    super::vasprintf::vasprintf(s, fmt, ap)
}
