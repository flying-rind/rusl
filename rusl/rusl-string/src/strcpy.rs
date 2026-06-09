//! strcpy — 将 src 字符串（含终止 null）复制到 dest 缓冲区。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strcpy — 将 src 字符串（含终止 null）复制到 dest 缓冲区。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 和 `src` 不重叠
/// - `dest` 至少可写 strlen(src) + 1 字节
/// - src 以 null 结尾
#[no_mangle]
pub extern "C" fn strcpy(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    // SAFETY: 调用者保证 dest 和 src 非空、不重叠，dest 有足够空间容纳 src（含 null 终止符），且 src 以 null 结尾。
    unsafe {
        let d = dest as *mut u8;
        let s = src as *const u8;
        let mut i = 0;
        loop {
            let byte = *s.add(i);
            *d.add(i) = byte;
            if byte == 0 {
                break;
            }
            i += 1;
        }
        dest
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strcpy_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let bytes = src.to_bytes_with_nul();
    let len = bytes.len();
    dest[..len].copy_from_slice(bytes);
    unsafe { dest.as_mut_ptr().add(len - 1) }
}
