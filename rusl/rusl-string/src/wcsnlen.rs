//! wcsnlen — 计算宽字符串 s 的长度，最多搜索 n 个宽字符。

#![allow(unused_imports, unused_variables)]

/// wcsnlen — 计算宽字符串 s 的长度，最多搜索 n 个宽字符。
///
/// # Safety
/// - `s` 非空
/// - 当 `n > 0` 时，s 至少可读 min(n, wcslen(s)+1) 个宽字符
#[no_mangle]
pub extern "C" fn wcsnlen(s: *const u32, n: usize) -> usize {
    // SAFETY: caller guarantees that s is non-null and, when n > 0,
    // s points to at least min(n, wcslen(s)+1) readable wide characters.
    unsafe {
        for i in 0..n {
            if *s.add(i) == 0 {
                return i;
            }
        }
        n
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsnlen_impl(s: &[u32], n: usize) -> usize {
    let limit = n.min(s.len());
    s[..limit].iter().position(|&c| c == 0).unwrap_or(limit)
}
