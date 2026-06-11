//! getchar_unlocked — 从标准输入 stdin 免锁读取一个字符。
//! 对应 musl src/stdio/getchar_unlocked.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

/// 从标准输入流 stdin 读取一个字符（不加锁）。调用者负责锁管理。
/// [Visibility]: User — POSIX 免锁扩展（需 _POSIX_C_SOURCE >= 200112L）。
#[no_mangle]
pub extern "C" fn getchar_unlocked() -> c_int {
    unimplemented!()
}
