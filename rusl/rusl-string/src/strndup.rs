//! strndup — 创建字符串 s 的副本，最多复制 n 个字符。通过 malloc 分配内存。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// strndup — 创建字符串 s 的副本，最多复制 n 个字符。通过 malloc 分配内存。
///
/// # Safety
/// - `s` 非空
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strndup(s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char {
    let src = s as *const u8;
    // 计算要复制的长度（不超过 n，不越过 null）
    let mut len = 0;
    while len < n {
        if unsafe { *src.add(len) } == 0 {
            break;
        }
        len += 1;
    }
    // 分配内存
    let mem = unsafe { malloc(len + 1) } as *mut u8;
    if mem.is_null() {
        return core::ptr::null_mut();
    }
    // 复制字节并添加 null
    for i in 0..len {
        unsafe { *mem.add(i) = *src.add(i); }
    }
    unsafe { *mem.add(len) = 0; }
    mem as *mut core::ffi::c_char
}

/// 安全的 Rust 内部实现。
pub(crate) fn strndup_impl(s: &core::ffi::CStr, n: usize) -> Option<*mut u8> {
    let src = s.to_bytes();
    let copy_len = n.min(src.len());
    let mem = unsafe { malloc(copy_len + 1) } as *mut u8;
    if mem.is_null() {
        return None;
    }
    for i in 0..copy_len {
        unsafe { *mem.add(i) = src[i]; }
    }
    unsafe { *mem.add(copy_len) = 0; }
    Some(mem)
}
