//! wcsncat — 将 s 中最多 n 个宽字符追加到 d 末尾，始终追加 L'\0'。

#![allow(unused_imports, unused_variables)]

/// wcsncat — 将 s 中最多 n 个宽字符追加到 d 末尾，始终追加 L'\0'。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - d 和 s 以 L'\0' 结尾
/// - d 缓冲区至少可容纳 (wcslen(d) + min(n, wcslen(s)) + 1) 个 wchar_t
#[no_mangle]
pub unsafe extern "C" fn wcsncat(d: *mut u32, s: *const u32, n: usize) -> *mut u32 {
    // 找到 d 结尾
    let mut i = 0;
    while unsafe { *d.add(i) } != 0 {
        i += 1;
    }
    // 复制最多 n 个字符
    let mut j = 0;
    while j < n {
        let ch = unsafe { *s.add(j) };
        unsafe { *d.add(i) = ch; }
        if ch == 0 { return d; }
        i += 1;
        j += 1;
    }
    // 始终添加 null 终止符
    unsafe { *d.add(i) = 0; }
    d
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsncat_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let null_pos = dst.iter().position(|&c| c == 0).unwrap_or(dst.len());
    let copy_len = n.min(src.iter().position(|&c| c == 0).unwrap_or(src.len()));
    let end = null_pos + copy_len;
    if end <= dst.len() {
        dst[null_pos..end].copy_from_slice(&src[..copy_len]);
        if end < dst.len() {
            dst[end] = 0;
        }
    }
    dst.as_mut_ptr()
}
