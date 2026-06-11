//! 对应 musl src/stdio/puts.c
//! 向 stdout 输出字符串并自动追加换行符

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 将 C 字符串 s 写入 stdout，随后写入换行符 '\n'
#[no_mangle]
pub extern "C" fn puts(s: *const c_char) -> c_int {
    unimplemented!()
}
