//! wcpcpy — 将 s 指向的宽字符串（含终止 L'\0'）复制到 d，返回 d 中终止 null 的位置。

#![allow(unused_imports, unused_variables)]

/// wcpcpy — 将 s 指向的宽字符串（含终止 L'\0'）复制到 d，返回 d 中终止 null 的位置。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 (wcslen(s) + 1) 个 wchar_t
/// - s 以 L'\0' 结尾
#[no_mangle]
pub unsafe extern "C" fn wcpcpy(d: *mut u32, s: *const u32) -> *mut u32 {
    let mut i = 0;
    loop {
        let ch = unsafe { *s.add(i) };
        unsafe { *d.add(i) = ch; }
        if ch == 0 { return d.add(i) as *mut u32; }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcpcpy_impl(dst: &mut [u32], src: &[u32]) -> *mut u32 {
    for (i, &ch) in src.iter().enumerate() {
        if i >= dst.len() { break; }
        dst[i] = ch;
        if ch == 0 { return unsafe { dst.as_mut_ptr().add(i) }; }
    }
    unsafe { dst.as_mut_ptr().add(dst.len() - 1) }
}
