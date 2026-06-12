//! dprintf — 向文件描述符格式化输出。
//! 对应 musl src/stdio/dprintf.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// dprintf — 向文件描述符 fd 格式化输出（可变参数版本）。
#[no_mangle]
pub unsafe extern "C" fn dprintf(fd: c_int, fmt: *const c_char, mut args: ...) -> c_int {
    let ap = &raw mut args as *mut super::stdio_impl::VaList;
    super::vdprintf::vdprintf(fd, fmt, ap)
}
