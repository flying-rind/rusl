//! strrchr — 在字符串 s 中从后向前查找字符 c 最后一次出现的位置（包括终止 null）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strrchr — 在字符串 s 中从后向前查找字符 c 最后一次出现的位置（包括终止 null）。
///
/// # Safety
/// - `s` 非空
/// - s 以 null 结尾
#[no_mangle]
pub extern "C" fn strrchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    let p = s as *const u8;
    let target = c as u8;
    let mut result: *mut core::ffi::c_char = core::ptr::null_mut();
    let mut i = 0;
    loop {
        let byte = unsafe { *p.add(i) };
        if byte == target {
            result = unsafe { p.add(i) } as *mut core::ffi::c_char;
        }
        if byte == 0 {
            break;
        }
        i += 1;
    }
    result
}

/// 安全的 Rust 内部实现。
pub(crate) fn strrchr_impl(s: &core::ffi::CStr, c: u8) -> *const u8 {
    let mut result: *const u8 = core::ptr::null();
    for (i, &b) in s.to_bytes_with_nul().iter().enumerate() {
        if b == c {
            result = unsafe { s.as_ptr().add(i) as *const u8 };
        }
    }
    result
}
