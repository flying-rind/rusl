//! wcsncmp — 比较两个宽字符串的前 n 个宽字符。返回值限定为 -1、0、1。

#![allow(unused_imports, unused_variables)]

/// wcsncmp — 比较两个宽字符串的前 n 个宽字符。返回值限定为 -1、0、1。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcsncmp(l: *const u32, r: *const u32, n: usize) -> core::ffi::c_int {
    // SAFETY: 调用者必须保证 l 和 r 非空且指向以 L'\0' 结尾的宽字符串。
    unsafe {
        for i in 0..n {
            let lv = *l.add(i);
            let rv = *r.add(i);
            if lv != rv {
                return if lv < rv { -1 } else { 1 };
            }
            if lv == 0 { return 0; }
        }
        0
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsncmp_impl(l: &[u32], r: &[u32], n: usize) -> core::ffi::c_int {
    let limit = n.min(l.len()).min(r.len());
    for i in 0..limit {
        let lv = l[i];
        let rv = r[i];
        if lv != rv {
            return if lv < rv { -1 } else { 1 };
        }
        if lv == 0 { return 0; }
    }
    0
}
