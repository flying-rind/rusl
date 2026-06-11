//! 对应 musl src/stdio/printf.c
//! 标准输出格式化函数，vfprintf(stdout, _: ...) 的可变参数包装

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 将格式化字符串输出到 stdout
#[no_mangle]
pub unsafe extern "C" fn printf(fmt: *const c_char, _: ...) -> c_int {
    loop {}
}
