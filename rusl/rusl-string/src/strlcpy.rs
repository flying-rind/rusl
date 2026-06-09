//! strlcpy — 将字符串 s 复制到大小为 n 的缓冲区 d 中，始终保证 null 终止（n > 0）。返回 strlen(s)。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use super::strlen::strlen;
/// strlcpy — 将字符串 s 复制到大小为 n 的缓冲区 d 中，始终保证 null 终止（n > 0）。返回 strlen(s)。
///
/// # Safety
/// - `d` 非空或 `n == 0`
/// - `s` 非空
/// - `d` 至少可写 n 字节（n > 0 时）
/// - s 以 null 结尾
#[no_mangle]
pub extern "C" fn strlcpy(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize {
    // SAFETY: 调用者保证 d 非空（n>0 时）且至少可写 n 字节，s 非空且以 null 结尾。
    unsafe {
        let dst = d as *mut u8;
        let src = s as *const u8;
        // 计算 src 长度
        let slen = strlen(s);
        if n == 0 {
            return slen;
        }
        // 复制到缓冲区（保留一个位置给 null）
        let copy_len = slen.min(n - 1);
        for i in 0..copy_len {
            *dst.add(i) = *src.add(i);
        }
        *dst.add(copy_len) = 0;
        slen
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strlcpy_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize {
    let src = s.to_bytes();
    let slen = src.len();
    if d.is_empty() {
        return slen;
    }
    let copy_len = slen.min(d.len() - 1);
    d[..copy_len].copy_from_slice(&src[..copy_len]);
    d[copy_len] = 0;
    slen
}
