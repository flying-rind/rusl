//! __fopen_rb_ca — 调用方分配 FILE（Caller-Allocated）的只读打开实现。
//! 对应 musl src/stdio/__fopen_rb_ca.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use super::stdio_impl::FILE;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_open: i64 = 2;
#[cfg(target_arch = "aarch64")]
const SYS_openat: i64 = 1024;

const O_RDONLY: i32 = 0;
const O_CLOEXEC: i32 = 0o2000000;

/// 内部函数：以只读方式打开文件，使用调用方提供的 FILE 内存和缓冲区。
/// [Visibility]: Internal (hidden) — 由 freopen 等内部调用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fopen_rb_ca(
    filename: *const c_char,
    f: *mut FILE,
    buf: *mut u8,
    len: usize,
) -> *mut FILE {
    unsafe {
        // 清零 FILE
        core::ptr::write_bytes(f as *mut u8, 0, core::mem::size_of::<FILE>());

        let f_ref = &mut *f;

        // 打开文件
        #[cfg(target_arch = "x86_64")]
        let fd = rusl_core::__syscall3(SYS_open, filename as i64, (O_RDONLY | O_CLOEXEC) as i64, 0) as i32;
        #[cfg(target_arch = "aarch64")]
        let fd = rusl_core::__syscall4(SYS_openat, -100, filename as i64, (O_RDONLY | O_CLOEXEC) as i64, 0) as i32;

        if fd < 0 {
            return core::ptr::null_mut();
        }

        f_ref.fd = fd;
        f_ref.flags = super::stdio_impl::F_NOWR | super::stdio_impl::F_PERM;
        f_ref.buf = buf.add(super::stdio_impl::UNGET);
        f_ref.buf_size = len - super::stdio_impl::UNGET;
        f_ref.read = Some(super::__stdio_read::__stdio_read);
        f_ref.seek = Some(super::__stdio_seek::__stdio_seek);
        f_ref.close = Some(super::__stdio_close::__stdio_close);
        f_ref.lock = -1;

        f
    }
}
