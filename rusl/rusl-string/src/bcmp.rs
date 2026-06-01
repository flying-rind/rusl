//! bcmp — 比较两个内存区域的前 n 个字节是否相等。对外导出 C ABI 兼容的 `bcmp` 符号供链接器使用。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// bcmp — 比较两个内存区域的前 n 个字节是否相等。对外导出 C ABI 兼容的 `bcmp` 符号供链接器使用。
///
/// # Safety
/// - `s1` 非空、`s2` 非空
/// - 当 `n > 0` 时，`s1` 和 `s2` 各自指向至少可读 n 字节的内存
#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const core::ffi::c_void, s2: *const core::ffi::c_void, n: usize) -> core::ffi::c_int {
    let a = s1 as *const u8;
    let b = s2 as *const u8;
    for i in 0..n {
        if unsafe { *a.add(i) } != unsafe { *b.add(i) } {
            return 1;
        }
    }
    0
}

/// 安全的 Rust 内部实现。
pub(crate) fn bcmp_impl(s1: &[u8], s2: &[u8]) -> bool {
    s1 == s2
}
