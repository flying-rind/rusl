//! memmove — 将 src 的前 n 字节复制到 dest，正确处理源和目标区域重叠的情况。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memmove — 将 src 的前 n 字节复制到 dest，正确处理源和目标区域重叠的情况。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 至少可写 n 字节，`src` 至少可读 n 字节
#[no_mangle]
pub extern "C" fn memmove(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, n: usize) -> *mut core::ffi::c_void {
    unsafe {
        // 不能使用 ptr::copy，因为它内部可能降级为 LLVM memmove intrinsic 调用我们的 memmove，导致无限递归。
        let d = dest as *mut u8;
        let s = src as *const u8;
        if (d as *const u8) < s || (d as *const u8) >= s.wrapping_add(n) {
            // 正向复制（不重叠）
            for i in 0..n {
                *d.add(i) = *s.add(i);
            }
        } else {
            // 反向复制（处理重叠）
            let mut i = n;
            while i > 0 {
                i -= 1;
                *d.add(i) = *s.add(i);
            }
        }
        dest
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn memmove_impl(dst: &mut [u8], src: &[u8]) {
    let len = dst.len().min(src.len());
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
