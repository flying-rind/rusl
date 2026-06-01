//! strspn — 计算 s 的起始段长度，该段中所有字符都属于集合 c。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strspn — 计算 s 的起始段长度，该段中所有字符都属于集合 c。
///
/// # Safety
/// - `s` 非空、`c` 非空
/// - s 和 c 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strspn(s: *const core::ffi::c_char, c: *const core::ffi::c_char) -> usize {
    let sp = s as *const u8;
    let accept = c as *const u8;
    let mut i = 0;
    loop {
        let sc = unsafe { *sp.add(i) };
        if sc == 0 {
            return i;
        }
        // 检查 sc 是否在 accept 中
        let mut found = false;
        let mut j = 0;
        loop {
            let ac = unsafe { *accept.add(j) };
            if ac == 0 {
                break;
            }
            if ac == sc {
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            return i;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strspn_impl(s: &core::ffi::CStr, accept: &core::ffi::CStr) -> usize {
    let s_bytes = s.to_bytes();
    let accept_bytes = accept.to_bytes();
    for (i, &ch) in s_bytes.iter().enumerate() {
        if !accept_bytes.contains(&ch) {
            return i;
        }
    }
    s_bytes.len()
}
