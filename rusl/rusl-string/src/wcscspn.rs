//! wcscspn — 计算 s 的起始段长度，该段中不包含宽字符串 c 中的任何宽字符。

#![allow(unused_imports, unused_variables)]

/// wcscspn — 计算 s 的起始段长度，该段中不包含宽字符串 c 中的任何宽字符。
///
/// # Safety
/// - `s` 非空、`c` 非空
/// - s 和 c 以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcscspn(s: *const u32, c: *const u32) -> usize {
    // SAFETY: 调用者保证 s 和 c 是非空的以 L'\0' 结尾的宽字符串指针
    unsafe {
        let mut i = 0;
        loop {
            let ch = *s.add(i);
            if ch == 0 { return i; }
            // 检查 ch 是否在 c 中
            let mut j = 0;
            loop {
                let rc = *c.add(j);
                if rc == 0 { break; }
                if rc == ch { return i; }
                j += 1;
            }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcscspn_impl(s: &[u32], reject: &[u32]) -> usize {
    for (i, &ch) in s.iter().enumerate() {
        if ch == 0 { return i; }
        if reject.contains(&ch) { return i; }
    }
    s.len()
}
