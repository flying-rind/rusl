//! wcsrchr — 在宽字符串 s 中从后向前查找宽字符 c 最后一次出现的位置（包括终止 L'\0'）。

#![allow(unused_imports, unused_variables)]

/// wcsrchr — 在宽字符串 s 中从后向前查找宽字符 c 最后一次出现的位置（包括终止 L'\0'）。
///
/// # Safety
/// - `s` 非空
/// - s 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcsrchr(s: *const u32, c: u32) -> *mut u32 {
    // SAFETY: 调用者保证 s 非空且以 L'\0' 结尾
    unsafe {
        let mut result: *mut u32 = core::ptr::null_mut();
        let mut i = 0;
        loop {
            let ch = *s.add(i);
            if ch == c {
                result = s.add(i) as *mut u32;
            }
            if ch == 0 {
                break;
            }
            i += 1;
        }
        result
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsrchr_impl(s: &[u32], c: u32) -> Option<*const u32> {
    let mut result = None;
    for (i, &ch) in s.iter().enumerate() {
        if ch == c {
            result = Some(unsafe { s.as_ptr().add(i) });
        }
        if ch == 0 { break; }
    }
    result
}
