//! vswprintf — 宽字符串格式化输出（va_list 版本）。
//! 对应 musl src/stdio/vswprintf.c
//!
//! 通过创建自定义只写 FILE 流（sw_write 回调），将 vfwprintf 的
//! 输出重定向到用户提供的宽字符缓冲区。
//! musl 截断时返回 -1（非 C99 标准）。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_uint};

/// vswprintf — 将格式化宽字符串写入缓冲区 s，最多 n 个宽字符。
///
/// - `s`: 目标宽字符缓冲区（n > 0 时）；n == 0 时可为 NULL
/// - `n`: 缓冲区大小（宽字符数，含 L'\0'）
/// - `fmt`: 宽字符格式字符串（const wchar_t *）
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为写入的宽字符数（不含 L'\0'）；截断或错误时返回 -1。
#[no_mangle]
pub extern "C" fn vswprintf(
    s: *mut c_uint,
    n: usize,
    fmt: *const c_uint,
    ap: *mut VaList,
) -> c_int {
    unimplemented!()
}
