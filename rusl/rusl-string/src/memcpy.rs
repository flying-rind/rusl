//! memcpy — 将 src 的前 n 字节复制到 dest，调用者保证不重叠。对外导出 C ABI 兼容的 `memcpy` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memcpy — 将 src 的前 n 字节复制到 dest，调用者保证不重叠。对外导出 C ABI 兼容的 `memcpy` 符号。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 和 `src` 不重叠（违反则行为未定义）
/// - `dest` 至少可写 n 字节，`src` 至少可读 n 字节
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void {
    // 不能使用 slice::copy_from_slice，因为它内部调用 ptr::copy_nonoverlapping，
    // 而 ptr::copy_nonoverlapping 可能降级为 LLVM memcpy intrinsic 调用我们的 memcpy，导致无限递归。
    let d = dest as *mut u8;
    let s = src as *const u8;
    for i in 0..n {
        unsafe { *d.add(i) = *s.add(i); }
    }
    dest
}

/// 安全的 Rust 内部实现。
pub(crate) fn memcpy_impl(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        *d = *s;
    }
}
