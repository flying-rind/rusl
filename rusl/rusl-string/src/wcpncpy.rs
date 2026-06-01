//! wcpncpy — 将 s 中最多 n 个宽字符复制到 d。若 s 长度小于 n，剩余用 L'\0' 填充。返回 d + min(wcslen(s), n)。

#![allow(unused_imports, unused_variables)]

/// wcpncpy — 将 s 中最多 n 个宽字符复制到 d。若 s 长度小于 n，剩余用 L'\0' 填充。返回 d + min(wcslen(s), n)。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 n 个 wchar_t
/// - s 以 L'\0' 结尾
#[no_mangle]
pub unsafe extern "C" fn wcpncpy(d: *mut u32, s: *const u32, n: usize) -> *mut u32 {
    let mut i = 0;
    while i < n {
        let ch = unsafe { *s.add(i) };
        unsafe { *d.add(i) = ch; }
        if ch == 0 {
            let null_pos = i;
            i += 1;
            while i < n {
                unsafe { *d.add(i) = 0; }
                i += 1;
            }
            return d.add(null_pos) as *mut u32;
        }
        i += 1;
    }
    d.add(n) as *mut u32
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcpncpy_impl(dst: &mut [u32], src: &[u32], n: usize) -> *mut u32 {
    let n_actual = n.min(dst.len());
    let src_len = src.iter().position(|&c| c == 0).unwrap_or(src.len());
    let copy_len = n_actual.min(src_len);
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
    for i in copy_len..n_actual {
        dst[i] = 0;
    }
    if copy_len < n_actual {
        unsafe { dst.as_mut_ptr().add(copy_len) }
    } else {
        unsafe { dst.as_mut_ptr().add(n_actual) }
    }
}
