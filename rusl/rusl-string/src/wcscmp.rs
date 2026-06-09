//! wcscmp — 比较两个宽字符串 l 和 r。

#![allow(unused_imports, unused_variables)]

/// wcscmp — 比较两个宽字符串 l 和 r。
///
/// # Safety
/// - `l` 非空、`r` 非空
/// - l 和 r 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcscmp(l: *const u32, r: *const u32) -> core::ffi::c_int {
    // SAFETY: 调用者保证 l 和 r 是非空指针，且指向以 L'\0' 结尾的宽字符串。
    unsafe {
        let mut i = 0;
        loop {
            let lv = *l.add(i);
            let rv = *r.add(i);
            if lv != rv {
                return if lv < rv { -1 } else { 1 };
            }
            if lv == 0 {
                return 0;
            }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcscmp_impl(l: &[u32], r: &[u32]) -> core::ffi::c_int {
    let limit = l.len().min(r.len());
    for i in 0..limit {
        let lv = l[i];
        let rv = r[i];
        if lv != rv {
            return if lv < rv { -1 } else { 1 };
        }
        if lv == 0 {
            return 0;
        }
    }
    0
}
