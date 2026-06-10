//! __toread — 激活 FILE 的读模式。
//! 对应 musl src/stdio/__toread.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 激活 FILE 的读模式，返回 0 成功或 EOF(=-1) 失败。
#[no_mangle]
pub unsafe extern "C" fn __toread(f: *mut FILE) -> c_int {
    let f = &mut *f;
    f.mode |= f.mode - 1;
    if f.wpos != f.wbase {
        if let Some(write_fn) = f.write {
            write_fn(f, core::ptr::null(), 0);
        }
    }
    f.wpos = core::ptr::null_mut();
    f.wbase = core::ptr::null_mut();
    f.wend = core::ptr::null_mut();
    if f.flags & F_NORD != 0 {
        f.flags |= F_ERR;
        return EOF;
    }
    f.rpos = f.buf;
    f.rend = unsafe { f.buf.add(f.buf_size) };
    if f.flags & F_EOF != 0 { EOF } else { 0 }
}
