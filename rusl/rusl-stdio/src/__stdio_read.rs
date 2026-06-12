//! 对应 musl src/stdio/__stdio_read.c
//! 内部 FILE 默认读操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_read: i64 = 0;
#[cfg(target_arch = "aarch64")]
const SYS_read: i64 = 63;

/// 从文件描述符读取数据到用户缓冲区
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    let f_ref = &mut *f;
    // 简化版本：直接使用 read 系统调用
    // musl 使用 readv 同时填充 buf 和内部缓冲区，这里简化为单次 read
    let cnt = rusl_core::__syscall3(SYS_read, f_ref.fd as i64, buf as i64, len as i64);
    if cnt <= 0 {
        if cnt < 0 {
            f_ref.flags |= F_ERR;
        } else {
            f_ref.flags |= F_EOF;
        }
        0
    } else {
        cnt as usize
    }
}
