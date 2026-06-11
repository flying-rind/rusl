//! vfscanf — 格式化输入核心引擎。
//! 对应 musl src/stdio/vfscanf.c
//!
//! scanf 家族所有函数的底层实现。字符级状态机逐字符解析格式串，
//! 支持位置参数（%n$）、赋值抑制（%*）、动态分配（%m）等扩展。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vfscanf — 从 FILE 流读取格式化输入。
///
/// - `f`: 输入 FILE 流
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub extern "C" fn vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unimplemented!()
}

/// __isoc99_vfscanf — vfscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unimplemented!()
}
