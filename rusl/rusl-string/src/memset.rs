//! memset — 将 dest 的前 n 字节全部设置为值 c（转为 u8）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memset — 将 dest 的前 n 字节全部设置为值 c（转为 u8）。
///
/// # Safety
/// - `dest` 非空
/// - `dest` 至少可写 n 字节
#[no_mangle]
pub extern "C" fn memset(dest: *mut core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void {
    // 不能使用 slice::fill，因为 fill 内部可能调用 LLVM memset intrinsic，进而调用我们的 memset 导致无限递归。
    let ptr = dest as *mut u8;
    let val = c as u8;
    for i in 0..n {
        unsafe { *ptr.add(i) = val; }
    }
    dest
}

/// 安全的 Rust 内部实现。
pub(crate) fn memset_impl(dst: &mut [u8], val: u8) {
    for b in dst.iter_mut() {
        *b = val;
    }
}
