//! strncat — 将 s 中最多 n 个字符追加到 d 末尾，始终追加终止 null。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strncat — 将 s 中最多 n 个字符追加到 d 末尾，始终追加终止 null。
///
/// # Safety
/// - `d` 非空、`s` 非空
/// - `d` 和 `s` 不重叠
/// - d 和 s 以 null 结尾
/// - d 缓冲区至少可容纳 strlen(d) + min(n, strlen(s)) + 1 字节
#[no_mangle]
pub extern "C" fn strncat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> *mut core::ffi::c_char {
    let dst = d as *mut u8;
    let src = s as *const u8;
    // 找到 dst 结尾
    let mut i = 0;
    while unsafe { *dst.add(i) } != 0 {
        i += 1;
    }
    // 复制最多 n 个字符
    let mut j = 0;
    while j < n {
        let byte = unsafe { *src.add(j) };
        unsafe { *dst.add(i) = byte; }
        if byte == 0 {
            return d;
        }
        i += 1;
        j += 1;
    }
    // 始终追加 null 终止符
    unsafe { *dst.add(i) = 0; }
    d
}

/// 安全的 Rust 内部实现。
pub(crate) fn strncat_impl(d: &mut [u8], s: &core::ffi::CStr, n: usize) -> *mut u8 {
    let src = s.to_bytes();
    let null_pos = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    let copy_len = n.min(src.len());
    let end = null_pos + copy_len;
    if end < d.len() {
        d[null_pos..end].copy_from_slice(&src[..copy_len]);
        d[end] = 0; // 始终添加 null 终止符
    }
    unsafe { d.as_mut_ptr().add(null_pos) }
}
