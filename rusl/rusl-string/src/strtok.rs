//! strtok — 从字符串 s 中提取下一个 token，分隔符为 sep 中的任意字符。使用静态内部指针维护状态（非线程安全）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 静态状态指针
static mut STRTOK_STATE: *mut u8 = core::ptr::null_mut();

/// strtok — 从字符串 s 中提取下一个 token，分隔符为 sep 中的任意字符。使用静态内部指针维护状态（非线程安全）。
///
/// # Safety
/// - 首次调用 `s` 非空，后续可传 null
/// - `sep` 非空，以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strtok(s: *mut core::ffi::c_char, sep: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let src = if s.is_null() {
        STRTOK_STATE
    } else {
        s as *mut u8
    };
    if src.is_null() {
        return core::ptr::null_mut();
    }
    // 跳过开头的分隔符
    let sep_bytes = sep as *const u8;
    let mut p = src;
    loop {
        if unsafe { *p } == 0 {
            STRTOK_STATE = core::ptr::null_mut();
            return core::ptr::null_mut();
        }
        // 检查是否分隔符
        let mut is_sep = false;
        let mut j = 0;
        loop {
            let sc = unsafe { *sep_bytes.add(j) };
            if sc == 0 {
                break;
            }
            if sc == unsafe { *p } {
                is_sep = true;
                break;
            }
            j += 1;
        }
        if !is_sep {
            break;
        }
        p = unsafe { p.add(1) };
    }
    let token = p;
    // 找到 token 结尾
    loop {
        if unsafe { *p } == 0 {
            STRTOK_STATE = core::ptr::null_mut();
            return token as *mut core::ffi::c_char;
        }
        // 检查是否分隔符
        let mut is_sep = false;
        let mut j = 0;
        loop {
            let sc = unsafe { *sep_bytes.add(j) };
            if sc == 0 {
                break;
            }
            if sc == unsafe { *p } {
                is_sep = true;
                break;
            }
            j += 1;
        }
        if is_sep {
            unsafe { *p = 0; }
            STRTOK_STATE = unsafe { p.add(1) };
            return token as *mut core::ffi::c_char;
        }
        p = unsafe { p.add(1) };
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strtok_impl(s: Option<&mut [u8]>, sep: &[u8]) -> Option<*mut u8> {
    let src = s.unwrap_or_else(|| {
        unsafe {
            if STRTOK_STATE.is_null() {
                return &mut [];
            }
            let len = super::strlen::strlen(STRTOK_STATE as *const core::ffi::c_char);
            core::slice::from_raw_parts_mut(STRTOK_STATE, len + 1)
        }
    });
    if src.is_empty() {
        return None;
    }
    // 跳过开头分隔符
    let start = src.iter().position(|&b| !sep.contains(&b))?;
    let remaining = &mut src[start..];
    // 找到下一个分隔符
    let end = remaining.iter().position(|&b| sep.contains(&b));
    match end {
        Some(pos) => {
            remaining[pos] = 0;
            let next_start = start + pos + 1;
            if next_start < src.len() {
                unsafe { STRTOK_STATE = src.as_mut_ptr().add(next_start); }
            } else {
                unsafe { STRTOK_STATE = core::ptr::null_mut(); }
            }
            Some(unsafe { src.as_mut_ptr().add(start) })
        }
        None => {
            unsafe { STRTOK_STATE = core::ptr::null_mut(); }
            Some(unsafe { src.as_mut_ptr().add(start) })
        }
    }
}
