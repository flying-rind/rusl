//! vsscanf — 字符串格式化输入（va_list 版本）。
//! 对应 musl src/stdio/vsscanf.c
//!
//! 通过构造最小伪 FILE 对象（免锁、只读、自定义 string_read 回调）
//! 并委托 vfscanf 实现。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vsscanf — 从内存中的 null 结尾字符串读取格式化输入。
///
/// - `s`: 源字符串（只读）
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为匹配并赋值的输入项数；输入失败时返回 EOF。
#[no_mangle]
pub extern "C" fn vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unimplemented!()
}

/// __isoc99_vsscanf — vsscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unimplemented!()
}
