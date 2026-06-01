//! memchr — 在 src 指向内存的前 n 字节中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `memchr` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memchr — 在 src 指向内存的前 n 字节中查找字符 c 第一次出现的位置。对外导出 C ABI 兼容的 `memchr` 符号。
///
/// # Safety
/// - `src` 非空
/// - 当 `n > 0` 时，`src` 至少可读 n 字节
#[no_mangle]
pub unsafe extern "C" fn memchr(src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void {
    let p = src as *const u8;
    for i in 0..n {
        if unsafe { *p.add(i) } == c as u8 {
            return p.add(i) as *mut core::ffi::c_void;
        }
    }
    core::ptr::null_mut()
}

/// 安全的 Rust 内部实现。
pub(crate) fn memchr_impl(buf: &[u8], c: u8) -> Option<usize> {
    buf.iter().position(|&b| b == c)
}
