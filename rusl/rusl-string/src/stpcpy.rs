//! stpcpy — 将 s 指向的字符串（含终止 null）复制到 d，返回 d 中终止 null 的位置。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// stpcpy — 将 s 指向的字符串（含终止 null）复制到 d，返回 d 中终止 null 的位置。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 strlen(s) + 1 字节
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn stpcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let dest = d as *mut u8;
    let src = s as *const u8;
    let mut i = 0;
    loop {
        let byte = unsafe { *src.add(i) };
        unsafe { *dest.add(i) = byte; }
        if byte == 0 {
            return unsafe { dest.add(i) as *mut core::ffi::c_char };
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn stpcpy_impl(dst: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let bytes = src.to_bytes_with_nul();
    let len = bytes.len();
    dst[..len].copy_from_slice(bytes);
    unsafe { dst.as_mut_ptr().add(len - 1) }
}
