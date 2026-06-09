//! memmem — 在长度为 k 的内存区域（haystack）h0 中查找长度为 l 的子序列（needle）n0 第一次出现的位置。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// memmem — 在长度为 k 的内存区域（haystack）h0 中查找长度为 l 的子序列（needle）n0 第一次出现的位置。
///
/// # Safety
/// - `h0` 非空或 `k == 0`
/// - `n0` 非空或 `l == 0`
/// - 当 `k > 0` 时，`h0` 至少可读 k 字节
/// - 当 `l > 0` 时，`n0` 至少可读 l 字节
#[no_mangle]
pub extern "C" fn memmem(h0: *const core::ffi::c_void, k: usize, n0: *const core::ffi::c_void, l: usize) -> *mut core::ffi::c_void {
    // SAFETY: 调用者确保指针有效；当 k>0 时 h0 至少可读 k 字节；当 l>0 时 n0 至少可读 l 字节
    unsafe {
        let haystack = h0 as *const u8;
        let needle = n0 as *const u8;
        if l == 0 {
            return h0 as *mut core::ffi::c_void;
        }
        if k < l {
            return core::ptr::null_mut();
        }
        let first = *needle;
        let max_i = k - l;
        let mut i = 0usize;
        while i <= max_i {
            if *haystack.add(i) == first {
                // 检查剩余部分
                let mut found = true;
                for j in 1..l {
                    if *haystack.add(i + j) != *needle.add(j) {
                        found = false;
                        break;
                    }
                }
                if found {
                    return haystack.add(i) as *mut core::ffi::c_void;
                }
            }
            i += 1;
        }
        core::ptr::null_mut()
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn memmem_impl(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}
