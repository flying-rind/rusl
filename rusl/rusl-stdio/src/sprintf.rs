//! 对应 musl src/stdio/sprintf.c
//! 字符串格式化输出函数（无边界检查），vsprintf(s, _: ...) 的可变参数包装

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 将格式化字符串写入用户提供的缓冲区 s（无边界检查）
#[no_mangle]
pub unsafe extern "C" fn sprintf(s: *mut c_char, fmt: *const c_char, _: ...) -> c_int {
    loop {}
}
