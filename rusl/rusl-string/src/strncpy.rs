//! strncpy — 将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余位置用 '\0' 填充。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strncpy — 将 s 中最多 n 个字符复制到 d。若 s 长度小于 n，剩余位置用 '\0' 填充。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - `d` 至少可写 n 字节
/// - s 以 null 结尾
#[no_mangle]
pub extern "C" fn strncpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char {
    let dst = d as *mut u8;
    let src = s as *const u8;
    let mut i = 0;
    // 复制最多 n 个字符
    while i < n {
        let byte = unsafe { *src.add(i) };
        unsafe { *dst.add(i) = byte; }
        if byte == 0 {
            // 剩余位置用 '\0' 填充
            i += 1;
            while i < n {
                unsafe { *dst.add(i) = 0; }
                i += 1;
            }
            return d;
        }
        i += 1;
    }
    d
}

/// 安全的 Rust 内部实现。
pub(crate) fn strncpy_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8 {
    let src = s.to_bytes();
    let copy_len = n.min(src.len());
    d[..copy_len].copy_from_slice(&src[..copy_len]);
    // 如果 src 长度小于 n，填充零
    if copy_len < n {
        d[copy_len..n].fill(0);
    }
    d.as_mut_ptr()
}
