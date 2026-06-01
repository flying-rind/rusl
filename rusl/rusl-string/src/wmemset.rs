//! wmemset — 将 d 指向的宽字符数组的前 n 个元素全部设置为宽字符 c。

#![allow(unused_imports, unused_variables)]

/// wmemset — 将 d 指向的宽字符数组的前 n 个元素全部设置为宽字符 c。
///
/// # Safety
/// - `d` 非空
/// - `d` 至少可写 n 个 wchar_t
#[no_mangle]
pub unsafe extern "C" fn wmemset(d: *mut u32, c: u32, n: usize) -> *mut u32 {
    for i in 0..n {
        unsafe { *d.add(i) = c; }
    }
    d
}

/// 安全的 Rust 内部实现。
pub(crate) fn wmemset_impl(dst: &mut [u32], c: u32) {
    for elem in dst.iter_mut() {
        *elem = c;
    }
}
