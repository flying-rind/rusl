//! strpbrk — 在字符串 s 中查找 b 中任意字符首次出现的位置。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strpbrk — 在字符串 s 中查找 b 中任意字符首次出现的位置。
///
/// # Safety
/// - `s` 非空、`b` 非空
/// - s 和 b 以 null 结尾
#[no_mangle]
pub extern "C" fn strpbrk(s: *const core::ffi::c_char, b: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    // SAFETY: 调用者保证 s 和 b 非空，且均以 null 结尾。
    unsafe {
        let sp = s as *const u8;
        let accept = b as *const u8;
        let mut i = 0;
        loop {
            let sc = *sp.add(i);
            if sc == 0 {
                return core::ptr::null_mut();
            }
            let mut j = 0;
            loop {
                let ac = *accept.add(j);
                if ac == 0 {
                    break;
                }
                if ac == sc {
                    return sp.add(i) as *mut core::ffi::c_char;
                }
                j += 1;
            }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strpbrk_impl(s: &core::ffi::CStr, accept: &core::ffi::CStr) -> Option<*const u8> {
    let s_bytes = s.to_bytes();
    let accept_bytes = accept.to_bytes();
    for (i, &ch) in s_bytes.iter().enumerate() {
        if accept_bytes.contains(&ch) {
            return Some(unsafe { s.as_ptr().add(i) as *const u8 });
        }
    }
    None
}
