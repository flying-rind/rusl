//! fsetpos — 将文件流位置恢复到先前由 fgetpos 保存的位置。
//! 对应 musl src/stdio/fsetpos.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;
use super::fgetpos::fpos_t;

const SEEK_SET: c_int = 0;

/// 将文件流定位到 *pos 所表示的位置（SEEK_SET 绝对偏移量）。
/// [Visibility]: User — ISO C <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fsetpos(f: *mut FILE, pos: *const fpos_t) -> c_int {
    unsafe {
        super::fseek::__fseeko(f, *pos, SEEK_SET)
    }
}
