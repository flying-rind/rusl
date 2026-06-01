//! memccpy — 从 src 复制字节到 dest，直到已复制 n 字节或遇到字符 c。若遇到 c，将其复制后停止，返回 dest 中 c 之后的下一个字节位置。对外导出 C ABI 兼容的 `memccpy` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memccpy — 从 src 复制字节到 dest，直到已复制 n 字节或遇到字符 c。若遇到 c，将其复制后停止，返回 dest 中 c 之后的下一个字节位置。对外导出 C ABI 兼容的 `memccpy` 符号。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 和 `src` 不重叠（restrict 约束）
/// - `dest` 至少可写 n 字节，`src` 至少可读 n 字节
#[no_mangle]
pub unsafe extern "C" fn memccpy(dest: *mut core::ffi::c_void, src: *const core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void {
    let d = dest as *mut u8;
    let s = src as *const u8;
    let target = c as u8;
    for i in 0..n {
        let byte = unsafe { *s.add(i) };
        unsafe { *d.add(i) = byte; }
        if byte == target {
            return unsafe { d.add(i + 1) as *mut core::ffi::c_void };
        }
    }
    core::ptr::null_mut()
}

/// 安全的 Rust 内部实现。
pub(crate) fn memccpy_impl<'a>(dst: &'a mut [u8], src: &[u8], c: u8) -> Option<&'a mut u8> {
    let len = dst.len().min(src.len());
    for i in 0..len {
        dst[i] = src[i];
        if src[i] == c {
            return Some(&mut dst[i + 1]);
        }
    }
    None
}
