//! strncasecmp — 忽略大小写比较两个 C 字符串的前 n 个字符。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};
use rusl_core::c_types::locale_t;

/// strncasecmp — 忽略大小写比较两个 C 字符串的前 n 个字符。
///
/// # Safety
/// - `_l` 非空、`_r` 非空
/// - _l 和 _r 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strncasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char, n: usize) -> core::ffi::c_int {
    let a = _l as *const u8;
    let b = _r as *const u8;
    for i in 0..n {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };
        let al = av.to_ascii_lowercase();
        let bl = bv.to_ascii_lowercase();
        if al != bl {
            return (al as i32) - (bl as i32);
        }
        if av == 0 {
            return 0;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn __strncasecmp_l(l: *const c_char, r: *const c_char, n: usize, _loc: locale_t) -> c_int {
    unsafe { strncasecmp(l, r, n) }
}

#[no_mangle]
pub unsafe extern "C" fn strncasecmp_l(l: *const c_char, r: *const c_char, n: usize, loc: locale_t) -> c_int {
    unsafe { __strncasecmp_l(l, r, n, loc) }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strncasecmp_impl(l: &[u8], r: &[u8], n: usize) -> core::ffi::c_int {
    let limit = n.min(l.len()).min(r.len());
    for i in 0..limit {
        let al = l[i].to_ascii_lowercase();
        let bl = r[i].to_ascii_lowercase();
        if al != bl {
            return (al as i32) - (bl as i32);
        }
        if l[i] == 0 {
            return 0;
        }
    }
    0
}
