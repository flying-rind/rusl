//! wmemcmp — 比较 l 和 r 指向的宽字符数组的前 n 个元素。返回值限定为 -1、0、1。

#![allow(unused_imports, unused_variables)]

/// wmemcmp — 比较 l 和 r 指向的宽字符数组的前 n 个元素。返回值限定为 -1、0、1。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 各自至少可读 n 个 wchar_t
#[no_mangle]
pub extern "C" fn wmemcmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int {
    // SAFETY: 调用者保证 l 和 r 非空且各自至少可读 n 个 wchar_t
    unsafe {
        for i in 0..n {
            let lv = *l.add(i);
            let rv = *r.add(i);
            if lv != rv {
                return if lv < rv { -1 } else { 1 };
            }
        }
        0
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wmemcmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    let limit = n.min(l.len()).min(r.len());
    for i in 0..limit {
        let lv = l[i];
        let rv = r[i];
        if lv != rv {
            return if lv < rv { -1 } else { 1 };
        }
    }
    0
}
