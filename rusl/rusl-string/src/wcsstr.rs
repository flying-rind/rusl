//! wcsstr — 在宽字符串 h（haystack）中查找子串 n（needle）首次出现的位置。

#![allow(unused_imports, unused_variables)]

/// wcsstr — 在宽字符串 h（haystack）中查找子串 n（needle）首次出现的位置。
///
/// # Safety
/// - `h` 非空、`n` 非空
/// - h 和 n 以 L'\0' 结尾
#[no_mangle]
pub unsafe extern "C" fn wcsstr(h: *const u32, n: *const u32) -> *mut u32 {
    let haystack = h;
    let needle = n;
    if unsafe { *needle } == 0 {
        return h as *mut u32;
    }
    let first = unsafe { *needle };
    let mut i = 0;
    loop {
        let hc = unsafe { *haystack.add(i) };
        if hc == 0 { return core::ptr::null_mut(); }
        if hc == first {
            let mut j = 1;
            loop {
                let nc = unsafe { *needle.add(j) };
                if nc == 0 { return haystack.add(i) as *mut u32; }
                if unsafe { *haystack.add(i + j) } != nc { break; }
                j += 1;
            }
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsstr_impl(haystack: &[u32], needle: &[u32]) -> Option<*const u32> {
    if needle.is_empty() || needle[0] == 0 {
        return Some(haystack.as_ptr());
    }
    let first = needle[0];
    for (i, &hc) in haystack.iter().enumerate() {
        if hc == 0 { break; }
        if hc == first {
            if haystack[i..].iter().zip(needle.iter()).all(|(&a, &b)| a == b) {
                return Some(unsafe { haystack.as_ptr().add(i) });
            }
        }
    }
    None
}
