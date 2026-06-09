//! swab — 将 _src 中的 n 个字节相邻两两交换后复制到 _dest。若 n 为奇数，最后 1 字节不处理。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// swab — 将 _src 中的 n 个字节相邻两两交换后复制到 _dest。若 n 为奇数，最后 1 字节不处理。
///
/// # Safety
/// - `_src` 非空、`_dest` 非空
/// - `_src` 和 `_dest` 不重叠
/// - `_src` 至少可读 n 字节
/// - `_dest` 至少可写 (n & !1) 字节
#[no_mangle]
pub extern "C" fn swab(_src: *const core::ffi::c_void, _dest: *mut core::ffi::c_void, n: isize) {
    if n <= 1 {
        return;
    }
    let src = _src as *const u8;
    let dst = _dest as *mut u8;
    let len = (n as usize) & !1; // 取偶数
    let mut i = 0;
    while i < len {
        let a = unsafe { *src.add(i) };
        let b = unsafe { *src.add(i + 1) };
        unsafe { *dst.add(i) = b; }
        unsafe { *dst.add(i + 1) = a; }
        i += 2;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn swab_impl(src: &[u8], dst: &mut [u8], n: usize) {
    let len = (n.min(src.len()).min(dst.len())) & !1;
    for i in (0..len).step_by(2) {
        dst[i] = src[i + 1];
        dst[i + 1] = src[i];
    }
}
