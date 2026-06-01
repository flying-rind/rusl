//! wmemcpy — 将 s 指向的宽字符数组的前 n 个元素复制到 d。调用者保证不重叠。

#![allow(unused_imports, unused_variables)]

/// wmemcpy — 将 s 指向的宽字符数组的前 n 个元素复制到 d。调用者保证不重叠。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 n 个 wchar_t
/// - `s` 至少可读 n 个 wchar_t
#[no_mangle]
pub unsafe extern "C" fn wmemcpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32 {
    for i in 0..n {
        unsafe { *d.add(i) = *s.add(i); }
    }
    d
}

/// 安全的 Rust 内部实现。
pub(crate) fn wmemcpy_impl(dst: &mut [u32], src: &[u32]) {
    let len = dst.len().min(src.len());
    for i in 0..len {
        dst[i] = src[i];
    }
}
