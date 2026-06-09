//! strcspn — 计算 s 的起始段长度，该段中不包含字符串 c 中的任何字符。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strcspn — 计算 s 的起始段长度，该段中不包含字符串 c 中的任何字符。
///
/// # Safety
/// - `s` 非空、`c` 非空
/// - s 和 c 以 null 结尾
#[no_mangle]
pub extern "C" fn strcspn(s: *const core::ffi::c_char, c: *const core::ffi::c_char) -> usize {
    let sp = s as *const u8;
    let reject = c as *const u8;
    let mut i = 0;
    loop {
        let sc = unsafe { *sp.add(i) };
        if sc == 0 {
            return i;
        }
        // 检查 sc 是否在 reject 中
        let mut j = 0;
        loop {
            let rc = unsafe { *reject.add(j) };
            if rc == 0 {
                break;
            }
            if rc == sc {
                return i;
            }
            j += 1;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strcspn_impl(s: &core::ffi::CStr, reject: &core::ffi::CStr) -> usize {
    let s_bytes = s.to_bytes();
    let reject_bytes = reject.to_bytes();
    for (i, &ch) in s_bytes.iter().enumerate() {
        if reject_bytes.contains(&ch) {
            return i;
        }
    }
    s_bytes.len()
}
