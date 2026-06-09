//! wcsspn — 计算 s 的起始段长度，该段中所有宽字符都属于集合 c。

#![allow(unused_imports, unused_variables)]

/// wcsspn — 计算 s 的起始段长度，该段中所有宽字符都属于集合 c。
///
/// # Safety
/// - `s` 非空、`c` 非空
/// - s 和 c 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcsspn(s: *const u32, c: *const u32) -> usize {
    // SAFETY: 调用者保证 s 和 c 均非空且以 L'\0' 结尾
    unsafe {
        let mut i = 0;
        loop {
            let ch = *s.add(i);
            if ch == 0 { return i; }
            // 检查 ch 是否在 accept 中
            let mut found = false;
            let mut j = 0;
            loop {
                let ac = *c.add(j);
                if ac == 0 { break; }
                if ac == ch { found = true; break; }
                j += 1;
            }
            if !found { return i; }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsspn_impl(s: &[u32], accept: &[u32]) -> usize {
    for (i, &ch) in s.iter().enumerate() {
        if ch == 0 { return i; }
        if !accept.contains(&ch) { return i; }
    }
    s.len()
}
