//! fopen / freopen — 文件打开与重定向。
//! 对应 musl src/stdio/fopen.c 和 src/stdio/freopen.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use super::stdio_impl::FILE;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_open: i64 = 2;
#[cfg(target_arch = "x86_64")]
const SYS_close: i64 = 3;
#[cfg(target_arch = "aarch64")]
const SYS_openat: i64 = 1024;
#[cfg(target_arch = "aarch64")]
const SYS_close: i64 = 57;

/// 根据文件名和模式打开文件，返回缓冲的 FILE 流。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fopen(
    filename: *const c_char,
    mode: *const c_char,
) -> *mut FILE {
    unsafe {
        let flags = super::__fmodeflags::__fmodeflags(mode);
        if flags < 0 {
            return core::ptr::null_mut();
        }

        // 打开文件
        #[cfg(target_arch = "x86_64")]
        let fd = rusl_core::__syscall3(SYS_open, filename as i64, flags as i64, 0o666) as i32;
        #[cfg(target_arch = "aarch64")]
        let fd = rusl_core::__syscall4(SYS_openat, -100, filename as i64, flags as i64, 0o666) as i32;

        if fd < 0 {
            return core::ptr::null_mut();
        }

        let f = super::__fdopen::__fdopen(fd, mode);
        if f.is_null() {
            #[cfg(target_arch = "x86_64")]
            { rusl_core::__syscall1(SYS_close, fd as i64); }
            #[cfg(target_arch = "aarch64")]
            { rusl_core::__syscall1(SYS_close, fd as i64); }
        }
        f
    }
}

/// 将已有 FILE 流重定向到新文件路径，或修改当前 fd 的模式。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn freopen(
    _filename: *const c_char,
    _mode: *const c_char,
    _f: *mut FILE,
) -> *mut FILE {
    // 简化实现：暂不支持
    core::ptr::null_mut()
}
