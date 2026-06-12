//! 对应 musl src/stdio/setvbuf.c
//! 所有缓冲设置函数的最终实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

const _IONBF: c_int = 2;
const _IOLBF: c_int = 1;
const _IOFBF: c_int = 0;

/// 设置 FILE 流的缓冲模式、缓冲区位置和大小
#[no_mangle]
pub extern "C" fn setvbuf(
    f: *mut FILE,
    buf: *mut c_char,
    type_: c_int,
    size: usize,
) -> c_int {
    unsafe {
        let f_ref = &mut *f;
        f_ref.lbf = EOF;

        if type_ == _IONBF {
            f_ref.buf_size = 0;
        } else if type_ == _IOLBF || type_ == _IOFBF {
            if !buf.is_null() && size >= UNGET {
                f_ref.buf = (buf as *mut u8).add(UNGET);
                f_ref.buf_size = size - UNGET;
            }
            if type_ == _IOLBF && f_ref.buf_size > 0 {
                f_ref.lbf = b'\n' as c_int;
            }
        } else {
            return -1;
        }

        f_ref.flags |= F_SVB;

        0
    }
}
