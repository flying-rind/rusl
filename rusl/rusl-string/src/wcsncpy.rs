//! wcsncpy — 将 s 中最多 n 个宽字符复制到 d。若 wcslen(s) < n，剩余用 L'\0' 填充。

#![allow(unused_imports, unused_variables)]

/// wcsncpy — 将 s 中最多 n 个宽字符复制到 d。若 wcslen(s) < n，剩余用 L'\0' 填充。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 n 个 wchar_t
/// - s 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcsncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32 {
    // SAFETY: 调用者保证 d 和 s 非空、不重叠，d 至少可写 n 个 wchar_t，s 以 L'\0' 结尾
    unsafe {
        let mut i = 0;
        while i < n {
            let ch = *s.add(i);
            *d.add(i) = ch;
            if ch == 0 {
                // 剩余填零
                i += 1;
                while i < n {
                    *d.add(i) = 0;
                    i += 1;
                }
                return d;
            }
            i += 1;
        }
        d
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let copy_len = n.min(src.iter().position(|&c| c == 0).unwrap_or(src.len()));
    let n_actual = n.min(dst.len());
    let copy_actual = copy_len.min(n_actual);
    dst[..copy_actual].copy_from_slice(&src[..copy_actual]);
    // 剩余填零
    for i in copy_actual..n_actual {
        dst[i] = 0;
    }
    dst.as_mut_ptr()
}
