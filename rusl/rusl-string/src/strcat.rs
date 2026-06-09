//! strcat — 将 src 字符串追加到 dest 字符串末尾（覆盖 dest 的终止 null），包括 src 的终止 null。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strcat — 将 src 字符串追加到 dest 字符串末尾（覆盖 dest 的终止 null），包括 src 的终止 null。
///
/// # Safety
/// - `dest` 非空、`src` 非空
/// - `dest` 和 `src` 不重叠
/// - dest 和 src 以 null 结尾
/// - dest 缓冲区至少可容纳 strlen(dest) + strlen(src) + 1 字节
#[no_mangle]
pub extern "C" fn strcat(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let d = dest as *mut u8;
    let s = src as *const u8;
    // 找到 dest 的结尾
    let mut i = 0;
    while unsafe { *d.add(i) } != 0 {
        i += 1;
    }
    // 复制 src 到结尾
    let mut j = 0;
    loop {
        let byte = unsafe { *s.add(j) };
        unsafe { *d.add(i) = byte; }
        if byte == 0 {
            break;
        }
        i += 1;
        j += 1;
    }
    dest
}

/// 安全的 Rust 内部实现。
pub(crate) fn strcat_impl(dest: &mut [u8], src: &core::ffi::CStr) -> *mut u8 {
    let src_bytes = src.to_bytes_with_nul();
    // 找到 dest 中 null 的位置
    let null_pos = dest.iter().position(|&b| b == 0).unwrap_or(dest.len());
    let end = null_pos + src_bytes.len();
    if end <= dest.len() {
        dest[null_pos..end].copy_from_slice(src_bytes);
    }
    unsafe { dest.as_mut_ptr().add(end - 1) }
}
