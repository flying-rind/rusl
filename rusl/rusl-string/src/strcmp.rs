//! strcmp — 比较两个 C 字符串 l 和 r 的字典序大小。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strcmp — 比较两个 C 字符串 l 和 r 的字典序大小。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 null 结尾
#[no_mangle]
pub extern "C" fn strcmp(l: *const core::ffi::c_char, r: *const core::ffi::c_char) -> core::ffi::c_int {
    // 不能使用 CStr::from_ptr，因为它内部调用 strlen，而 strlen 被我们的实现覆盖。
    let a = l as *const u8;
    let b = r as *const u8;
    for i in 0.. {
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
pub(crate) fn strcmp_impl(l: &core::ffi::CStr, r: &core::ffi::CStr) -> core::ffi::c_int {
    for (a, b) in l.to_bytes().iter().zip(r.to_bytes().iter()) {
        if a != b { return (*a as i32) - (*b as i32); }
    }
    (l.count_bytes() as i32) - (r.count_bytes() as i32)
}
