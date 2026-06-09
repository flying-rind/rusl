//! wcscat — 将 src 宽字符串追加到 dest 宽字符串末尾。

#![allow(unused_imports, unused_variables)]

/// wcscat — 将 src 宽字符串追加到 dest 宽字符串末尾。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 和 `src` 不重叠
/// - dest 和 src 以 L'\0' 结尾
/// - dest 缓冲区至少可容纳 (wcslen(dest) + wcslen(src) + 1) 个 wchar_t
#[no_mangle]
pub extern "C" fn wcscat(dest: *mut u32, src: *const u32) -> *mut u32 {
    // SAFETY: 调用者保证 dest 和 src 非空、不重叠、以 L'\0' 结尾，
    // 且 dest 缓冲区有足够空间容纳追加后的字符串。
    unsafe {
        // 找到 dest 结尾
        let mut i = 0;
        while *dest.add(i) != 0 {
            i += 1;
        }
        // 复制 src
        let mut j = 0;
        loop {
            let ch = *src.add(j);
            *dest.add(i) = ch;
            if ch == 0 {
                break;
            }
            i += 1;
            j += 1;
        }
        dest
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcscat_impl(dest: &mut [u32], src: &[u32]) -> *mut u32 {
    let null_pos = dest.iter().position(|&c| c == 0).unwrap_or(dest.len());
    let remaining = &mut dest[null_pos..];
    for (i, &ch) in src.iter().enumerate() {
        if i >= remaining.len() { break; }
        remaining[i] = ch;
        if ch == 0 { break; }
    }
    dest.as_mut_ptr()
}
