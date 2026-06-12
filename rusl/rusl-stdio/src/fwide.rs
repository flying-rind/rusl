//! fwide — 设置/查询 FILE 流的宽字符/字节方向（orientation）。
//! 对应 musl src/stdio/fwide.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;

/// 设置或查询 FILE 流 f 的方向。
/// mode > 0: 设为宽字符模式; mode < 0: 设为字节模式; mode == 0: 仅查询。
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fwide(f: *mut FILE, mode: c_int) -> c_int {
    unsafe {
        let f_ref = &mut *f;
        if mode != 0 {
            if f_ref.mode == 0 {
                f_ref.mode = if mode > 0 { 1 } else { -1 };
            }
        }
        f_ref.mode
    }
}
