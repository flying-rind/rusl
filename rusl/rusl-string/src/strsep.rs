//! strsep — 从 *str 中提取下一个 token，分隔符为 sep 中的任意字符。可正确处理空 token。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// strsep — 从 *str 中提取下一个 token，分隔符为 sep 中的任意字符。可正确处理空 token。
///
/// # Safety
/// - `str` 非空
/// - `sep` 非空，以 null 结尾
/// - 若 *str 非 null，*str 以 null 结尾
#[no_mangle]
pub extern "C" fn strsep(str: *mut *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    // SAFETY: caller guarantees `str` is non-null, `sep` is non-null and null-terminated,
    // and if `*str` is non-null, it is null-terminated.
    unsafe {
        let s = *str;
        if s.is_null() {
            return core::ptr::null_mut();
        }
        // 找到第一个不在分隔符中的字符（跳过开头的分隔符）
        let sp = s as *const u8;
        let sep_bytes = sep as *const u8;
        // 构建分隔符集合（简单实现，直接遍历比较）
        let token_start = sp;
        let mut found_token = false;
        // 在字符串中扫描
        let mut i = 0;
        loop {
            let c = *sp.add(i);
            if c == 0 {
                // 字符串结束
                if !found_token {
                    // 没有找到 token
                    *str = core::ptr::null_mut();
                    return core::ptr::null_mut();
                }
                // 找到最后一个 token（不含分隔符）
                *str = core::ptr::null_mut();
                return s as *mut core::ffi::c_char;
            }
            // 检查 c 是否是分隔符
            let mut is_sep = false;
            let mut j = 0;
            loop {
                let sc = *sep_bytes.add(j);
                if sc == 0 {
                    break;
                }
                if sc == c {
                    is_sep = true;
                    break;
                }
                j += 1;
            }
            if is_sep {
                if found_token {
                    // 找到 token 的结尾，替换分隔符为 null
                    *(sp.add(i) as *mut u8) = 0;
                    *str = sp.add(i + 1) as *mut core::ffi::c_char;
                    return s;
                }
                // 跳过开头的分隔符
            } else {
                if !found_token {
                    found_token = true;
                    // token_start 已经在 sp+i 处
                }
            }
            i += 1;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strsep_impl(str: &mut Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8> {
    let s_ptr = str.as_mut().map(|s| s.as_mut_ptr())?;
    let s = unsafe { core::slice::from_raw_parts_mut(s_ptr, str.as_ref().unwrap().len()) };
    // 跳过开头分隔符
    let start = s.iter().position(|&b| !sep.contains(&b))?;
    // 找到下一个分隔符或结尾
    let end = s[start..].iter().position(|&b| sep.contains(&b));
    match end {
        Some(pos) => {
            let token_end = start + pos;
            if token_end < s.len() {
                s[token_end] = 0; // replace separator with null
            }
            let result = s_ptr.wrapping_add(start);
            *str = None;
            Some(result)
        }
        None => {
            let result = s_ptr.wrapping_add(start);
            *str = None;
            Some(result)
        }
    }
}
