//! strcasecmp — 忽略大小写比较两个 C 字符串 _l 和 _r。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};
use rusl_core::c_types::locale_t;

/// strcasecmp — 忽略大小写比较两个 C 字符串 _l 和 _r。
///
/// # Safety
/// - `_l` 非空、`_r` 非空
/// - _l 和 _r 以 null 结尾
#[no_mangle]
pub extern "C" fn strcasecmp(_l: *const core::ffi::c_char, _r: *const core::ffi::c_char) -> core::ffi::c_int {
    let a = _l as *const u8;
    let b = _r as *const u8;
    let mut i = 0;
    loop {
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
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn __strcasecmp_l(l: *const c_char, r: *const c_char, _loc: locale_t) -> c_int {
    strcasecmp(l, r)
}

#[no_mangle]
pub extern "C" fn strcasecmp_l(l: *const c_char, r: *const c_char, loc: locale_t) -> c_int {
    unsafe { __strcasecmp_l(l, r, loc) }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strcasecmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int {
    let limit = l.len().min(r.len());
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
