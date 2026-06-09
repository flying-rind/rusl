//! wcslen — 计算宽字符串 s 的长度（不含终止 L'\0'）。

#![allow(unused_imports, unused_variables)]

/// wcslen — 计算宽字符串 s 的长度（不含终止 L'\0'）。
///
/// # Safety
/// - `s` 非空
/// - s 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcslen(s: *const u32) -> usize {
    // SAFETY: caller guarantees s is non-null and points to a null-terminated wide string.
    unsafe {
        let mut i = 0;
        while *s.add(i) != 0 {
            i += 1;
        }
        i
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcslen_impl(s: &[u32]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}
