//! rindex — 在字符串 s 中从后向前查找字符 c 最后一次出现的位置。等价于 strrchr。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// rindex — 在字符串 s 中从后向前查找字符 c 最后一次出现的位置。等价于 strrchr。
///
/// # Safety
/// - `s` 非空
/// - s 指向以 null 结尾的有效 C 字符串
#[no_mangle]
pub extern "C" fn rindex(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    // SAFETY: 调用者保证 s 指向有效的以 null 结尾的 C 字符串
    unsafe {
        // rindex 等价于 strrchr（从后向前查找）
        // 直接实现以避免额外的符号依赖
        let p = s as *const u8;
        let target = c as u8;
        let mut result: *mut core::ffi::c_char = core::ptr::null_mut();
        let mut i = 0;
        loop {
            let byte = *p.add(i);
            if byte == target {
                result = p.add(i) as *mut core::ffi::c_char;
            }
            if byte == 0 {
                break;
            }
            i += 1;
        }
        result
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn rindex_impl(s: &core::ffi::CStr, c: u8) -> Option<*const core::ffi::c_char> {
    let mut result = None;
    for (i, &b) in s.to_bytes_with_nul().iter().enumerate() {
        if b == c {
            result = Some(unsafe { s.as_ptr().add(i) });
        }
    }
    result
}
