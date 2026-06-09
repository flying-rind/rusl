//! memcmp — 比较 vl 和 vr 指向内存区域的前 n 个字节。对外导出 C ABI 兼容的 `memcmp` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memcmp — 比较 vl 和 vr 指向内存区域的前 n 个字节。对外导出 C ABI 兼容的 `memcmp` 符号。
///
/// # Safety
/// - `vl` 非空、`vr` 非空
/// - 当 `n > 0` 时，两者各自至少可读 n 字节
#[no_mangle]
pub extern "C" fn memcmp(vl: *const core::ffi::c_void, vr: *const core::ffi::c_void, n: usize) -> core::ffi::c_int {
    // 使用原始指针比较，避免任何可能调用 memcmp intrinsic 的 Rust 操作。
    let a = vl as *const u8;
    let b = vr as *const u8;
    for i in 0..n {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };
        if av != bv {
            return (av as i32) - (bv as i32);
        }
    }
    0
}

/// 安全的 Rust 内部实现。
pub(crate) fn memcmp_impl(a: &[u8], b: &[u8]) -> core::cmp::Ordering {
    for (x, y) in a.iter().zip(b.iter()) {
        match x.cmp(y) {
            core::cmp::Ordering::Equal => continue,
            ne => return ne,
        }
    }
    core::cmp::Ordering::Equal
}
