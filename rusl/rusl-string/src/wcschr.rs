//! wcschr — 在宽字符串 s 中查找宽字符 c 首次出现的位置（包括终止 L'\0'）。

#![allow(unused_imports, unused_variables)]

/// wcschr — 在宽字符串 s 中查找宽字符 c 首次出现的位置（包括终止 L'\0'）。
///
/// 调用者必须保证 `s` 非空且以 L'\0' 结尾。
#[no_mangle]
pub extern "C" fn wcschr(s: *const u32, c: u32) -> *mut u32 {
    // SAFETY: 调用者保证 s 非空且以 L'\0' 结尾，指针运算和读取均在合法范围内。
    unsafe {
        let mut i = 0;
        loop {
            let ch = *s.add(i);
            if ch == c {
                return s.add(i) as *mut u32;
            }
            if ch == 0 {
                return core::ptr::null_mut();
            }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcschr_impl(s: &[u32], c: u32) -> Option<*const u32> {
    for (i, &ch) in s.iter().enumerate() {
        if ch == c {
            return Some(unsafe { s.as_ptr().add(i) });
        }
        if ch == 0 { break; }
    }
    None
}
