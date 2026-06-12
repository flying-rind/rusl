//! 对应 musl src/stdio/__overflow.c
//! 内部输出缓冲区溢出处理实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 处理 stdio 输出缓冲区的"溢出"情况
#[no_mangle]
pub(crate) unsafe extern "C" fn __overflow(f: *mut FILE, _c: c_int) -> c_int {
    let f = &mut *f;
    let c = _c as u8;

    if f.wend.is_null() && super::__towrite::__towrite(f) != 0 {
        return EOF;
    }
    if f.wpos != f.wend && c != f.lbf as u8 {
        *f.wpos = c;
        f.wpos = f.wpos.add(1);
        return c as c_int;
    }
    if f.write.map_or(0, |write| write(f, &c as *const u8, 1)) != 1 {
        return EOF;
    }
    c as c_int
}
