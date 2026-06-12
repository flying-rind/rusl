//! 对应 musl src/stdio/setlinebuf.c
//! GNU 扩展，将 FILE 流设为行缓冲模式

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

const _IOLBF: i32 = 1;

/// 将流 f 设为行缓冲模式，等价于 setvbuf(f, NULL, _IOLBF, 0)
#[no_mangle]
pub extern "C" fn setlinebuf(f: *mut FILE) {
    super::setvbuf::setvbuf(f, core::ptr::null_mut(), _IOLBF, 0);
}
