//! wcsncasecmp — 忽略大小写比较两个宽字符串的前 n 个宽字符。

#![allow(unused_imports, unused_variables)]

/// wcsncasecmp — 忽略大小写比较两个宽字符串的前 n 个宽字符。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 L'\0' 结尾
/// 宽字符转 ASCII 小写
fn wchar_to_lower(c: u32) -> u32 {
    if c >= b'A' as u32 && c <= b'Z' as u32 {
        c + (b'a' as u32 - b'A' as u32)
    } else {
        c
    }
}

pub extern "C" fn wcsncasecmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int {
    for i in 0..n {
        let lv = unsafe { *l.add(i) };
        let rv = unsafe { *r.add(i) };
        let ll = wchar_to_lower(lv);
        let rl = wchar_to_lower(rv);
        if ll != rl {
            return if ll < rl { -1 } else { 1 };
        }
        if lv == 0 { return 0; }
    }
    0
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsncasecmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    let limit = n.min(l.len()).min(r.len());
    for i in 0..limit {
        let ll = wchar_to_lower(l[i]);
        let rl = wchar_to_lower(r[i]);
        if ll != rl {
            return if ll < rl { -1 } else { 1 };
        }
        if l[i] == 0 { return 0; }
    }
    0
}
