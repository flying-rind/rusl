//! index — 在字符串 s 中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `index` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// index — 在字符串 s 中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `index` 符号。
///
/// # Safety
/// - `s` 非空，指向以 null 结尾的有效 C 字符串
#[no_mangle]
pub unsafe extern "C" fn index(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    // index 等价于 strchr
    super::strchr::strchr(s, c)
}

/// 安全的 Rust 内部实现。
pub(crate) fn index_impl(s: &core::ffi::CStr, c: u8) -> Option<*const u8> {
    super::strchr::strchr_impl(s, c).map(|p| p as *const u8)
}
