//! __towrite — 激活 FILE 的写模式。
//! 对应 musl src/stdio/__towrite.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 激活 FILE 的写模式，返回 0 成功或 EOF(=-1) 失败。
#[no_mangle]
pub unsafe extern "C" fn __towrite(f: *mut FILE) -> c_int {
    let f = &mut *f;
    f.mode |= f.mode - 1; // 无论初始值为何，结果均为 -1
    if f.flags & F_NOWR != 0 {
        f.flags |= F_ERR;
        return EOF;
    }
    f.rpos = core::ptr::null_mut();
    f.rend = core::ptr::null_mut();
    f.wpos = f.buf;
    f.wbase = f.buf;
    f.wend = if f.buf_size > 0 {
        unsafe { f.buf.add(f.buf_size) }
    } else {
        core::ptr::null_mut()
    };
    0
}
