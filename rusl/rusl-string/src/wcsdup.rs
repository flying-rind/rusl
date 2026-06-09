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

pub extern "C" fn wcsdup(s: *const u32) -> *mut u32 {
    // SAFETY: musl libc 对外 API；调用者保证 `s` 指向以 L'\0' 结尾的有效宽字符串，
    // 且对返回的堆内存有独占访问权。
    unsafe {
        let mut len = 0;
        while *s.add(len) != 0 {
            len += 1;
        }
        let mem = malloc((len + 1) * 4) as *mut u32;
        if mem.is_null() {
            return core::ptr::null_mut();
        }
        for i in 0..=len {
            *mem.add(i) = *s.add(i);
        }
        mem
    }
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
