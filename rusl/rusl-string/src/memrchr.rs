//! memrchr — 在 m 指向内存区域的前 n 字节中从后向前查找字符 c 最后一次出现的位置。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memrchr — 在 m 指向内存区域的前 n 字节中从后向前查找字符 c 最后一次出现的位置。
///
/// # Safety
/// - `m` 非空
/// - 当 `n > 0` 时，`m` 至少可读 n 字节
#[no_mangle]
pub unsafe extern "C" fn __memrchr(m: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void {
    let p = m as *const u8;
    let mut i = n;
    while i > 0 {
        i -= 1;
        if unsafe { *p.add(i) } == c as u8 {
            return p.add(i) as *mut core::ffi::c_void;
        }
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn memrchr(m: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void {
    // SAFETY: delegates to __memrchr which performs bounds-checked raw pointer reads
    unsafe { __memrchr(m, c, n) }
}

/// 安全的 Rust 内部实现。
pub(crate) fn memrchr_impl(buf: &[u8], byte: u8) -> Option<usize> {
    buf.iter().rposition(|&b| b == byte)
}
