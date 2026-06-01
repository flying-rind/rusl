//! wcscpy — 将 s 指向的宽字符串（含终止 L'\0'）复制到 d。

#![allow(unused_imports, unused_variables)]

/// wcscpy — 将 s 指向的宽字符串（含终止 L'\0'）复制到 d。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 (wcslen(s) + 1) 个 wchar_t
/// - s 以 L'\0' 结尾
#[no_mangle]
pub unsafe extern "C" fn wcscpy(d: *mut u32, s: *const u32) -> *mut u32 {
    let mut i = 0;
    loop {
        let ch = unsafe { *s.add(i) };
        unsafe { *d.add(i) = ch; }
        if ch == 0 {
            return d;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcscpy_impl(dst: &mut [u32], src: &[u32]) -> *mut u32 {
    for (i, &ch) in src.iter().enumerate() {
        if i < dst.len() {
            dst[i] = ch;
        }
        if ch == 0 {
            return dst.as_mut_ptr();
        }
    }
    dst.as_mut_ptr()
}
