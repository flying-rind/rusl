//! wcswcs — 在宽字符串 haystack 中查找子串 needle 首次出现的位置。wcsstr 的 BSD 别名。

#![allow(unused_imports, unused_variables)]

/// wcswcs — 在宽字符串 haystack 中查找子串 needle 首次出现的位置。wcsstr 的 BSD 别名。
///
/// # Safety
/// - `haystack` 非空、`needle` 非空
/// - haystack 和 needle 以 L'\0' 结尾
use super::wcsstr::wcsstr;

pub unsafe extern "C" fn wcswcs(haystack: *const u32, needle: *const u32) -> *mut u32 {
    // wcswcs 是 wcsstr 的 BSD 别名
    wcsstr(haystack, needle)
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcswcs_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32> {
    super::wcsstr::wcsstr_impl(haystack, needle)
}
