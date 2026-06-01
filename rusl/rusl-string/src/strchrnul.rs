//! strchrnul — 在字符串 s 中查找字符 c 首次出现的位置。若未找到，返回指向终止 null 的指针。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strchrnul — 在字符串 s 中查找字符 c 首次出现的位置。若未找到，返回指向终止 null 的指针。
///
/// # Safety
/// - `s` 非空
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strchrnul(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    let p = s as *const u8;
    let target = c as u8;
    let mut i = 0;
    loop {
        let byte = unsafe { *p.add(i) };
        if byte == target || byte == 0 {
            return p.add(i) as *mut core::ffi::c_char;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strchrnul_impl(s: &core::ffi::CStr, c: u8) -> *const u8 {
    for (i, &b) in s.to_bytes_with_nul().iter().enumerate() {
        if b == c || b == 0 {
            return unsafe { s.as_ptr().add(i) as *const u8 };
        }
    }
    unsafe { s.as_ptr().add(s.count_bytes()) as *const u8 }
}
