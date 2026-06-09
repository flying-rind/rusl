//! wmemmove — 将 s 指向的宽字符数组的前 n 个元素复制到 d，正确处理重叠。

#![allow(unused_imports, unused_variables)]

/// wmemmove — 将 s 指向的宽字符数组的前 n 个元素复制到 d，正确处理重叠。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 至少可写 n 个 wchar_t
/// - `s` 至少可读 n 个 wchar_t
#[no_mangle]
pub extern "C" fn wmemmove(d: *mut u32, s: *const u32, n: usize) -> *mut u32 {
    // SAFETY: 调用者保证 d 和 s 非空且可读/可写 n 个 wchar_t。
    unsafe {
        let dst = d;
        let src = s;
        if (dst as *const u32) < src || (dst as *const u32) >= src.wrapping_add(n) {
            for i in 0..n {
                *dst.add(i) = *src.add(i);
            }
        } else {
            let mut i = n;
            while i > 0 {
                i -= 1;
                *dst.add(i) = *src.add(i);
            }
        }
        d
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wmemmove_impl(dst: &mut [u32], src: &[u32]) {
    let len = dst.len().min(src.len());
    if dst.as_ptr() <= src.as_ptr() || dst.as_ptr() >= unsafe { src.as_ptr().add(len) } {
        for i in 0..len {
            dst[i] = src[i];
        }
    } else {
        let mut i = len;
        while i > 0 {
            i -= 1;
            dst[i] = src[i];
        }
    }
}
