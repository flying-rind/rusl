//! wcsdup — 创建宽字符串 s 的堆副本。

#![allow(unused_imports, unused_variables)]

/// wcsdup — 创建宽字符串 s 的堆副本。
///
/// # Safety
/// - `s` 非空
/// - s 以 L'\0' 结尾
use core::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub unsafe extern "C" fn wcsdup(s: *const u32) -> *mut u32 {
    // 计算长度
    let mut len = 0;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    let mem = unsafe { malloc((len + 1) * 4) } as *mut u32;
    if mem.is_null() { return core::ptr::null_mut(); }
    for i in 0..=len {
        unsafe { *mem.add(i) = *s.add(i); }
    }
    mem
}

/// 安全的 Rust 内部实现。
pub(crate) fn wcsdup_impl(s: &[u32]) -> Option<*mut u32> {
    let len = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    let mem = unsafe { malloc((len + 1) * 4) } as *mut u32;
    if mem.is_null() { return None; }
    for i in 0..=len {
        unsafe { *mem.add(i) = s[i]; }
    }
    Some(mem)
}
