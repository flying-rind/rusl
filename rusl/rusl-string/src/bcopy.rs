//! bcopy — 将 s1 的前 n 字节复制到 s2，支持源与目标重叠。对外导出 C ABI 兼容的 `bcopy` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// bcopy — 将 s1 的前 n 字节复制到 s2，支持源与目标重叠。对外导出 C ABI 兼容的 `bcopy` 符号。
///
/// # Safety
/// - `s1` 非空、`s2` 非空
/// - 当 `n > 0` 时，`s1` 至少可读 n 字节，`s2` 至少可写 n 字节
#[no_mangle]
pub unsafe extern "C" fn bcopy(s1: *const core::ffi::c_void, s2: *mut core::ffi::c_void, n: usize) {
    // 不能使用 ptr::copy 因为可能降级为 LLVM memmove 导致递归。
    let src = s1 as *const u8;
    let dst = s2 as *mut u8;
    if (dst as *const u8) < src || (dst as *const u8) >= src.wrapping_add(n) {
        for i in 0..n {
            unsafe { *dst.add(i) = *src.add(i); }
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            unsafe { *dst.add(i) = *src.add(i); }
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn bcopy_impl(src: &[u8], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    if dst.as_ptr() <= src.as_ptr() || dst.as_ptr() >= unsafe { src.as_ptr().add(len) } {
        for i in 0..len {
            dst[i] = src[i];
        }
    } else {
        let mut i = len;
        while i > 0 {
            i -= 1;
            dst[i] = src[i];
        }
    }
}
