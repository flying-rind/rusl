//! vdprintf — 文件描述符格式化输出（va_list 版本，POSIX 扩展）。
//! 对应 musl src/stdio/vdprintf.c
//!
//! 通过构造最小伪 FILE 对象，委托 vfprintf 实现。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vfprintf::vfprintf;
use core::ffi::{c_char, c_int};

/// vdprintf — 向文件描述符 fd 格式化输出。
///
/// 在栈上构造的最小伪 FILE：
/// - fd = 目标描述符
/// - lbf = EOF（无行缓冲）
/// - write = __stdio_write（直接系统调用）
/// - buf_size = 0（强制每个输出都通过 write 系统调用）
/// - lock = -1（禁用锁，栈上对象无并发风险）
#[no_mangle]
pub extern "C" fn vdprintf(fd: c_int, fmt: *const c_char, ap: *mut VaList) -> c_int {
    // SAFETY: 在栈上构造 FILE 并委托 vfprintf。因 buf_size=0 强制每次
    // printf_core 调用 out() 都直接走 write syscall，无需缓冲区。
    unsafe {
        let mut f: FILE = core::mem::zeroed();
        f.fd = fd;
        f.lbf = EOF;
        f.write = Some(super::__stdio_write::__stdio_write);
        // musl trick: 用 fmt 的一个字节地址作为 dummy buf 指针，
        // 使 buf 不为 null；但 buf_size=0 强制走 write 路径。
        f.buf = fmt as *mut u8;
        f.buf_size = 0;
        f.lock = -1;

        vfprintf(&mut f as *mut FILE, fmt, ap)
    }
}
