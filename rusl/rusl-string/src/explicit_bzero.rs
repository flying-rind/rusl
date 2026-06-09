//! explicit_bzero — 将 d 指向内存的前 n 字节全部置零，并通过编译器屏障阻止优化移除清零操作。用于安全擦除敏感数据。对外导出 C ABI 兼容的 `explicit_bzero` 符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
/// explicit_bzero — 将 d 指向内存的前 n 字节全部置零，并通过编译器屏障阻止优化移除清零操作。用于安全擦除敏感数据。对外导出 C ABI 兼容的 `explicit_bzero` 符号。
///
/// # Safety
/// - `d` 非空
/// - 当 `n > 0` 时，`d` 至少可写 n 字节
#[no_mangle]
pub extern "C" fn explicit_bzero(d: *mut core::ffi::c_void, n: usize) {
    // SAFETY: 调用者保证 d 非空且指向至少 n 可写字节
    unsafe {
        let p = d as *mut u8;
        for i in 0..n {
            // 使用 volatile 写入阻止编译器优化清零操作
            core::ptr::write_volatile(p.add(i), 0);
        }
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn explicit_bzero_impl(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        unsafe { core::ptr::write_volatile(b as *mut u8, 0); }
    }
}
