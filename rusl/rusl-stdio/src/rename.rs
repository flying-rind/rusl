//! 对应 musl src/stdio/rename.c
//! 将文件或目录从旧路径重命名为新路径

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

#[cfg(target_arch = "x86_64")]
const SYS_rename: i64 = 82;
#[cfg(target_arch = "aarch64")]
const SYS_renameat: i64 = 38;

/// 将文件系统对象从 old 路径重命名为 new 路径
#[no_mangle]
pub extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        { let ret = rusl_core::__syscall2(SYS_rename, old as i64, new as i64); if ret < 0 { -1 } else { 0 } }
        #[cfg(target_arch = "aarch64")]
        { let ret = rusl_core::__syscall3(SYS_renameat, -100, old as i64, -100, new as i64); if ret < 0 { -1 } else { 0 } }
    }
}
