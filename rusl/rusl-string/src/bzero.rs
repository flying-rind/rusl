//! bzero — 将 s 指向内存的前 n 字节全部置零。对外导出 C ABI 兼容的 `bzero` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// bzero — 将 s 指向内存的前 n 字节全部置零。对外导出 C ABI 兼容的 `bzero` 符号。
///
/// # Safety
/// - `s` 非空
/// - 当 `n > 0` 时，`s` 至少可写 n 字节
#[no_mangle]
pub extern "C" fn bzero(s: *mut core::ffi::c_void, n: usize) {
    // SAFETY: caller ensures s is non-null and points to at least n writable bytes
    unsafe {
        let p = s as *mut u8;
        for i in 0..n {
            *p.add(i) = 0;
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn bzero_impl(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        *b = 0;
    }
}
