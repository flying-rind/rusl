//! 对应 musl src/stdio/sscanf.c
//! 字符串格式化输入函数，vsscanf(s, _: ...) 的可变参数包装

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 从内存中的 null 结尾字符串 s 读取格式化输入
#[no_mangle]
pub unsafe extern "C" fn sscanf(s: *const c_char, fmt: *const c_char, _: ...) -> c_int {
    loop {}
}

/// __isoc99_sscanf — sscanf 的弱别名，C99 兼容
#[no_mangle]
pub unsafe extern "C" fn __isoc99_sscanf(s: *const c_char, fmt: *const c_char, _: ...) -> c_int {
    loop {}
}
