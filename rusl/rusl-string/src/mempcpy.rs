//! mempcpy — 将 src 的前 n 字节复制到 dest，返回 dest + n（最后一个写入字节之后的位置）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// mempcpy — 将 src 的前 n 字节复制到 dest，返回 dest + n（最后一个写入字节之后的位置）。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 至少可写 n 字节，`src` 至少可读 n 字节
/// - `dest` 和 `src` 不重叠
#[no_mangle]
pub extern "C" fn mempcpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void {
    // SAFETY: mempcpy 调用者保证 dest 和 src 非空、内存区域有效且不重叠。
    unsafe {
        let d = dest as *mut u8;
        let s = src as *const u8;
        for i in 0..n {
            *d.add(i) = *s.add(i);
        }
        d.add(n) as *mut core::ffi::c_void
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn mempcpy_impl<'a>(dst: &'a mut [u8], src: &[u8]) -> &'a mut [u8] {
    let len = dst.len().min(src.len());
    for i in 0..len {
        dst[i] = src[i];
    }
    &mut dst[len..]
}
