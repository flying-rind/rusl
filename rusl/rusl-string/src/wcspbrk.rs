//! wcspbrk — 在宽字符串 s 中查找 b 中任意宽字符首次出现的位置。

#![allow(unused_imports, unused_variables)]

/// wcspbrk — 在宽字符串 s 中查找 b 中任意宽字符首次出现的位置。
///
/// # Safety
/// - `s` 非空、`b` 非空
/// - s 和 b 以 L'\0' 结尾
#[no_mangle]
pub unsafe extern "C" fn wcspbrk(s: *const u32, b: *const u32) -> *mut u32 {
    let mut i = 0;
    loop {
        let ch = unsafe { *s.add(i) };
        if ch == 0 { return core::ptr::null_mut(); }
        // 检查 ch 是否在 b 中
        let mut j = 0;
        loop {
            let bc = unsafe { *b.add(j) };
            if bc == 0 { break; }
            if bc == ch { return s.add(i) as *mut u32; }
            j += 1;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcspbrk_impl(s: &[u32], accept: &[u32]) -> Option<*const u32> {
    for (i, &ch) in s.iter().enumerate() {
        if ch == 0 { break; }
        if accept.iter().any(|&a| a == ch) {
            return Some(unsafe { s.as_ptr().add(i) });
        }
    }
    None
}
