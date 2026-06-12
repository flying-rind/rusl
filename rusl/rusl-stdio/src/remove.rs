//! 对应 musl src/stdio/remove.c
//! 删除指定路径的文件或空目录

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

#[cfg(target_arch = "x86_64")]
const SYS_unlink: i64 = 87;
#[cfg(target_arch = "aarch64")]
const SYS_unlinkat: i64 = 35;

/// 从文件系统中删除 path 指向的文件
#[no_mangle]
pub extern "C" fn remove(path: *const c_char) -> c_int {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        { let ret = rusl_core::__syscall1(SYS_unlink, path as i64); if ret < 0 { -1 } else { 0 } }
        #[cfg(target_arch = "aarch64")]
        { let ret = rusl_core::__syscall2(SYS_unlinkat, -100, path as i64); if ret < 0 { -1 } else { 0 } }
    }
}
