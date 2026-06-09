//! wcstok — 从宽字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态（可重入版本）。

#![allow(unused_imports, unused_variables)]

/// wcstok — 从宽字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态（可重入版本）。
///
/// # Safety
/// - `p` 非空
/// - 首次调用 `s` 非空，后续可传 null
/// - `sep` 非空，以 L'\0' 结尾
#[no_mangle]
pub extern "C" fn wcstok(s: *mut u32, sep: *const u32, p: *mut *mut u32) -> *mut u32 {
    // SAFETY: 调用者保证 p 非空，s/sep 满足文档前置条件；内部指针运算仅在对齐的宽字符数组范围内进行。
    unsafe {
        let src = if s.is_null() { *p } else { s };
        if src.is_null() { return core::ptr::null_mut(); }
        // 跳过开头分隔符
        let mut pos = src;
        loop {
            if *pos == 0 {
                *p = core::ptr::null_mut();
                return core::ptr::null_mut();
            }
            let mut is_sep = false;
            let mut j = 0;
            loop {
                let sc = *sep.add(j);
                if sc == 0 { break; }
                if sc == *pos { is_sep = true; break; }
                j += 1;
            }
            if !is_sep { break; }
            pos = pos.add(1);
        }
        let token = pos;
        loop {
            if *pos == 0 {
                *p = core::ptr::null_mut();
                return token;
            }
            let mut is_sep = false;
            let mut j = 0;
            loop {
                let sc = *sep.add(j);
                if sc == 0 { break; }
                if sc == *pos { is_sep = true; break; }
                j += 1;
            }
            if is_sep {
                *pos = 0;
                *p = pos.add(1);
                return token;
            }
            pos = pos.add(1);
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcstok_impl(state: &mut Option<*mut u32>, sep: &[u32]) -> Option<*mut u32> {
    let src = (*state)?;
    let s = unsafe { core::slice::from_raw_parts_mut(src, super::wcslen::wcslen(src) + 1) };
    let start = s.iter().position(|&c| !sep.contains(&c))?;
    let remaining = &mut s[start..];
    let end = remaining.iter().position(|&c| sep.contains(&c));
    match end {
        Some(pos) => {
            remaining[pos] = 0;
            if start + pos + 1 < s.len() {
                *state = Some(unsafe { s.as_mut_ptr().add(start + pos + 1) });
            } else {
                *state = None;
            }
            Some(unsafe { s.as_mut_ptr().add(start) })
        }
        None => {
            *state = None;
            Some(unsafe { s.as_mut_ptr().add(start) })
        }
    }
}
