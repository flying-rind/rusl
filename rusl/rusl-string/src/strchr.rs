//! strchr — 在字符串 s 中查找字符 c 首次出现的位置（包括终止 null）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strchr — 在字符串 s 中查找字符 c 首次出现的位置（包括终止 null）。
///
/// # Safety
/// - `s` 非空
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    let p = s as *const u8;
    let target = c as u8;
    let mut i = 0;
    loop {
        let byte = unsafe { *p.add(i) };
        if byte == target {
            return p.add(i) as *mut core::ffi::c_char;
        }
        if byte == 0 {
            return core::ptr::null_mut();
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strchr_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char> {
    for (i, &b) in s.to_bytes_with_nul().iter().enumerate() {
        if b == c {
            return Some(unsafe { s.as_ptr().add(i) });
        }
    }
    None
}
