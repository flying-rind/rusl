//! wcscasecmp_l — 在指定 locale 下忽略大小写比较两个宽字符串。musl 实现中 locale 参数被忽略。

#![allow(unused_imports, unused_variables)]

/// wcscasecmp_l — 在指定 locale 下忽略大小写比较两个宽字符串。musl 实现中 locale 参数被忽略。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 L'\0' 结尾
use super::wcscasecmp::wcscasecmp;

pub unsafe extern "C" fn wcscasecmp_l(l: *const u32, r: *const u32, _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    // musl 实现忽略 locale 参数
    wcscasecmp(l, r)
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcscasecmp_l_impl(l: &[u32], r: &[u32], _locale: *mut core::ffi::c_void) -> core::ffi::c_int {
    super::wcscasecmp::wcscasecmp_impl(l, r)
}
