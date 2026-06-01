//! strlen — 计算以 null 结尾的 UTF-8/C 字符串的长度（字节数，不含 null 终止符）。对外导出 C ABI 兼容的 `strlen` 符号供链接器使用。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
/// strlen — 计算以 null 结尾的 UTF-8/C 字符串的长度（字节数，不含 null 终止符）。对外导出 C ABI 兼容的 `strlen` 符号供链接器使用。
///
/// # Safety
/// - `s` 为非空指针（`!s.is_null()`）
/// - `s` 指向以 `\0` 终止的有效字节序列
/// - 若作为 `&CStr` 使用则要求字节序列为有效 UTF-8 子集（ASCII 或按平台约定）
#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const core::ffi::c_char) -> usize {
    // 注意：不能使用 CStr::from_ptr，因为它内部会调用我们的 strlen 导致无限递归。
    // 采用简单逐字节扫描。
    let s = s as *const u8;
    let mut i = 0usize;
    while unsafe { *s.add(i) } != 0 {
        i += 1;
    }
    i
}

/// 安全的 Rust 内部实现。
pub(crate) fn str_len(s: &core::ffi::CStr) -> usize {
    s.to_bytes().len()
}
