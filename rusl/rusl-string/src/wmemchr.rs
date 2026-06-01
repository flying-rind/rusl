//! wmemchr — 在 s 指向的宽字符数组的前 n 个元素中查找宽字符 c 首次出现的位置。

#![allow(unused_imports, unused_variables)]

/// wmemchr — 在 s 指向的宽字符数组的前 n 个元素中查找宽字符 c 首次出现的位置。
///
/// # Safety
/// - `s` 非空
/// - `s` 至少可读 n 个 wchar_t
#[no_mangle]
pub unsafe extern "C" fn wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32 {
    for i in 0..n {
        if unsafe { *s.add(i) } == c {
            return s.add(i) as *mut u32;
        }
    }
    core::ptr::null_mut()
}

/// 安全的 Rust 内部实现。
pub(crate) fn wmemchr_impl(s: &[u32], c: u32, n: usize) -> Option<*const u32> {
    let limit = n.min(s.len());
    s[..limit].iter().position(|&x| x == c)
        .map(|i| unsafe { s.as_ptr().add(i) })
}
