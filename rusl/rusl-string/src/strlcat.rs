//! strlcat — 将字符串 s 追加到大小为 n 的缓冲区 d 中已有字符串之后，始终保证 null 终止（n > 0）。返回所需的总大小。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use super::strlen::strlen;
/// strlcat — 将字符串 s 追加到大小为 n 的缓冲区 d 中已有字符串之后，始终保证 null 终止（n > 0）。返回所需的总大小。
///
/// # Safety
/// - `d` 非空或 `n == 0`
/// - `s` 非空
/// - d 以 null 结尾
/// - s 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strlcat(d: *mut core::ffi::c_char, s: *const core::ffi::c_char, n: usize) -> usize {
    let dst = d as *mut u8;
    let src = s as *const u8;
    // 找到 dst 中 null 的位置（或 n 限制位置）
    let mut dlen = 0;
    while dlen < n && unsafe { *dst.add(dlen) } != 0 {
        dlen += 1;
    }
    if dlen == n {
        // dst 没有 null 终止符
        return dlen + unsafe { strlen(s) };
    }
    // 计算 src 长度，同时复制
    let mut slen = 0;
    while slen < n - dlen {
        let byte = unsafe { *src.add(slen) };
        unsafe { *dst.add(dlen + slen) = byte; }
        if byte == 0 {
            return dlen + slen;
        }
        slen += 1;
    }
    // 到达缓冲区边界，添加 null 终止符
    unsafe { *dst.add(n - 1) = 0; }
    // 继续计算 src 剩余长度
    let mut total = dlen + slen;
    while unsafe { *src.add(slen) } != 0 {
        total += 1;
        slen += 1;
    }
    total
}

/// 安全的 Rust 内部实现。
pub(crate) fn strlcat_impl(d: &mut [u8], s: &core::ffi::CStr) -> usize {
    let src = s.to_bytes();
    let dlen = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    let space = d.len().saturating_sub(dlen);
    let copy_len = src.len().min(space.saturating_sub(1));
    if copy_len > 0 {
        d[dlen..dlen + copy_len].copy_from_slice(&src[..copy_len]);
    }
    if dlen < d.len() {
        d[dlen + copy_len] = 0;
    }
    dlen + src.len()
}
