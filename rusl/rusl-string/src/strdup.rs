//! strdup — 创建字符串 s 的堆副本，调用者负责释放。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::ffi::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// strdup — 创建字符串 s 的堆副本，调用者负责释放。
///
/// # Safety
/// - `s` 非空
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strdup(s: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let src = s as *const u8;
    // 计算长度
    let mut len = 0;
    while unsafe { *src.add(len) } != 0 {
        len += 1;
    }
    // 分配内存
    let mem = unsafe { malloc(len + 1) } as *mut u8;
    if mem.is_null() {
        return core::ptr::null_mut();
    }
    // 复制字节（含 null）
    for i in 0..=len {
        unsafe { *mem.add(i) = *src.add(i); }
    }
    mem as *mut core::ffi::c_char
}

/// 安全的 Rust 内部实现。
pub(crate) fn strdup_impl(s: &core::ffi::CStr) -> Option<*mut u8> {
    let bytes = s.to_bytes_with_nul();
    let mem = unsafe { malloc(bytes.len()) } as *mut u8;
    if mem.is_null() {
        return None;
    }
    for i in 0..bytes.len() {
        unsafe { *mem.add(i) = bytes[i]; }
    }
    Some(mem)
}
