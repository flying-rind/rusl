//! strtok_r — strtok 的可重入版本。从字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// strtok_r — strtok 的可重入版本。从字符串 s 中提取下一个 token，使用调用者提供的指针 *p 维护状态。
///
/// # Safety
/// - `p` 非空
/// - 首次调用 `s` 非空，后续可传 null
/// - `sep` 非空，以 null 结尾
#[no_mangle]
pub extern "C" fn strtok_r(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char, p: *mut *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    // SAFETY: 调用者保证 p 非空；首次调用时 s 非空且指向有效 C 字符串；sep 非空且以 null 结尾且指向有效 C 字符串。
    // *p 指向 s 内部某处（首次调用）或先前位置（后续调用），均为合法可读写的字节指针。
    unsafe {
        let src = if s.is_null() {
            *p as *mut u8
        } else {
            s as *mut u8
        };
        if src.is_null() {
            return core::ptr::null_mut();
        }
        // 跳过开头的分隔符
        let sep_bytes = sep as *const u8;
        let mut pos = src;
        loop {
            if *pos == 0 {
                *p = core::ptr::null_mut();
                return core::ptr::null_mut();
            }
            let mut is_sep = false;
            let mut j = 0;
            loop {
                let sc = *sep_bytes.add(j);
                if sc == 0 { break; }
                if sc == *pos {
                    is_sep = true;
                    break;
                }
                j += 1;
            }
            if !is_sep { break; }
            pos = pos.add(1);
        }
        let token = pos;
        // 找到 token 结尾
        loop {
            if *pos == 0 {
                *p = core::ptr::null_mut();
                return token as *mut core::ffi::c_char;
            }
            let mut is_sep = false;
            let mut j = 0;
            loop {
                let sc = *sep_bytes.add(j);
                if sc == 0 { break; }
                if sc == *pos {
                    is_sep = true;
                    break;
                }
                j += 1;
            }
            if is_sep {
                *pos = 0;
                *p = pos.add(1) as *mut core::ffi::c_char;
                return token as *mut core::ffi::c_char;
            }
            pos = pos.add(1);
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strtok_r_impl(state: &mut Option<*mut u8>, sep: &[u8]) -> Option<*mut u8> {
    let src = (*state)?;
    let s = unsafe { core::slice::from_raw_parts_mut(src, super::strlen::strlen(src as *const core::ffi::c_char) + 1) };
    if s.is_empty() {
        return None;
    }
    // 跳过开头分隔符
    let start = s.iter().position(|&b| !sep.contains(&b))?;
    let remaining = &mut s[start..];
    let end = remaining.iter().position(|&b| sep.contains(&b));
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
