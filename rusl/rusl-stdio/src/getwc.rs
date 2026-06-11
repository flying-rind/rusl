//! getwc — 从 FILE 流读取一个宽字符。等价于 fgetwc(f)。
//! 对应 musl src/stdio/getwc.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_uint;
use crate::stdio_impl::FILE;

/// 从 FILE 流 f 中读取一个宽字符（wchar_t）。
/// 等价于 fgetwc(f)。
/// [Visibility]: User — <wchar.h> / <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn getwc(f: *mut FILE) -> c_uint /* wint_t */ {
    unimplemented!()
}
