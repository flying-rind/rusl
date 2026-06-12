//! vscanf — 标准输入格式化读取（va_list 版本）。
//! 对应 musl src/stdio/vscanf.c
//!
//! 直接委托 vfscanf(stdin, _: ...) 实现，纯转发代理。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vscanf — 从 stdin 读取格式化输入。
///
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub extern "C" fn vscanf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    let f = unsafe { super::stdin::stdin };
    super::vfscanf::vfscanf(f, fmt, ap)
}

/// __isoc99_vscanf — vscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vscanf(fmt: *const c_char, ap: *mut VaList) -> c_int {
    vscanf(fmt, ap)
}
