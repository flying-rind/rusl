//! vasprintf — 动态分配缓冲区的格式化输出（va_list 版本，GNU 扩展）。
//! 对应 musl src/stdio/vasprintf.c
//!
//! 采用两阶段策略：先干跑计算长度，再 malloc 分配缓冲区并写入。
//! 返回值由 malloc 分配，调用者负责 free。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;
use core::ffi::c_int;

/// vasprintf — 将格式化字符串写入动态分配的缓冲区。
///
/// - `s`: 输出参数，成功时指向 malloc 分配的 null 结尾字符串
/// - `fmt`: 格式字符串
/// - `ap`: va_list 参数列表
///
/// 返回值：成功时为格式化字符串长度（不含 '\0'）；失败时返回 -1。
#[no_mangle]
pub extern "C" fn vasprintf(
    s: *mut *mut c_char,
    fmt: *const c_char,
    ap: *mut VaList,
) -> c_int {
    unimplemented!()
}
