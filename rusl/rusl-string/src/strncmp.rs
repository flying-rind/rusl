//! strncmp — 比较两个 C 字符串的前 n 个字符。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strncmp — 比较两个 C 字符串的前 n 个字符。
///
/// # Safety
/// - `_l` 非空、`_r` 非空
/// - _l 和 _r 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strncmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int {
    let a = _l as *const u8;
    let b = _r as *const u8;
    for i in 0..n {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };
        if av != bv {
            return (av as i32) - (bv as i32);
        }
        if av == 0 {
            return 0;
        }
    }
    0
}

/// 安全的 Rust 内部实现。
pub(crate) fn strncmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int {
    let limit = n.min(l.len()).min(r.len());
    for i in 0..limit {
        let av = l[i];
        let bv = r[i];
        if av != bv {
            return (av as i32) - (bv as i32);
        }
        if av == 0 {
            return 0;
        }
    }
    // 如果在截止前一个字符串结束（遇到 null），但另一个更长
    if limit < n {
        if l.len() > limit && l[limit] == 0 {
            return 0;
        }
        if r.len() > limit && r[limit] == 0 {
            return 0;
        }
    }
    0
}
