//! strcasestr — 在字符串 h（haystack）中忽略大小写查找子串 n（needle）首次出现的位置。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strcasestr — 在字符串 h（haystack）中忽略大小写查找子串 n（needle）首次出现的位置。
///
/// # Safety
/// - `h` 非空、`n` 非空
/// - h 和 n 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strcasestr(h: *const core::ffi::c_char, n: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let haystack = h as *const u8;
    let needle = n as *const u8;
    // 空 needle 返回 haystack
    if unsafe { *needle } == 0 {
        return h as *mut core::ffi::c_char;
    }
    let first = unsafe { *needle }.to_ascii_lowercase();
    let mut i = 0;
    loop {
        let hc = unsafe { *haystack.add(i) };
        if hc == 0 {
            return core::ptr::null_mut();
        }
        if hc.to_ascii_lowercase() == first {
            // 比较剩余部分（忽略大小写）
            let mut j = 1;
            loop {
                let nc = unsafe { *needle.add(j) };
                if nc == 0 {
                    return haystack.add(i) as *mut core::ffi::c_char;
                }
                let hc2 = unsafe { *haystack.add(i + j) };
                if hc2.to_ascii_lowercase() != nc.to_ascii_lowercase() {
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strcasestr_impl(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    // 忽略大小写的比较函数
    let eq_ignore_case = |a: u8, b: u8| a.to_ascii_lowercase() == b.to_ascii_lowercase();
    haystack
        .windows(needle.len())
        .position(|w| w.iter().zip(needle.iter()).all(|(a, b)| eq_ignore_case(*a, *b)))
}
