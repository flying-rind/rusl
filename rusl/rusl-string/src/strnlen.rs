//! strnlen — 计算字符串 s 的长度，最多搜索 n 个字符。若在 n 个字符内未找到 '\0'，返回 n。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strnlen — 计算字符串 s 的长度，最多搜索 n 个字符。若在 n 个字符内未找到 '\0'，返回 n。
///
/// # Safety
/// - `s` 非空
/// - 当 `n > 0` 时，s 至少可读 min(n, strlen(s)+1) 字节
#[no_mangle]
pub unsafe extern "C" fn strnlen(s: *const core::ffi::c_char, n: usize) -> usize {
    let p = s as *const u8;
    for i in 0..n {
        if unsafe { *p.add(i) } == 0 {
            return i;
        }
    }
    n
}

/// 安全的 Rust 内部实现。
pub(crate) fn strnlen_impl(s: &[u8], n: usize) -> usize {
    let limit = n.min(s.len());
    s[..limit].iter().position(|&b| b == 0).unwrap_or(limit)
}
