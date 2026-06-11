//! 对应 musl src/stdio/scanf.c
//! 标准输入格式化读取函数，vscanf(fmt, _: ...) 的可变参数包装

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 从 stdin 读取格式化输入
#[no_mangle]
pub unsafe extern "C" fn scanf(fmt: *const c_char, _: ...) -> c_int {
    loop {}
}

/// __isoc99_scanf — scanf 的弱别名，C99 兼容
#[no_mangle]
pub unsafe extern "C" fn __isoc99_scanf(fmt: *const c_char, _: ...) -> c_int {
    loop {}
}
