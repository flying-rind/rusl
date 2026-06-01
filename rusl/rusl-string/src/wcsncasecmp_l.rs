//! wcsncasecmp_l — 在指定 locale 下忽略大小写比较两个宽字符串的前 n 个宽字符。musl 实现忽略 locale 参数。

#![allow(unused_imports, unused_variables)]

/// wcsncasecmp_l — 在指定 locale 下忽略大小写比较两个宽字符串的前 n 个宽字符。musl 实现忽略 locale 参数。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 L'\0' 结尾
use super::wcsncasecmp::wcsncasecmp;

pub unsafe extern "C" fn wcsncasecmp_l(l: *const u32, r: *const u32, n: usize, _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    // musl 实现忽略 locale 参数
    wcsncasecmp(l, r, n)
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsncasecmp_l_impl(l: &[u32], r: &[u32], n: usize, _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    super::wcsncasecmp::wcsncasecmp_impl(l, r, n)
}
