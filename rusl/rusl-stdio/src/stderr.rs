//! 对应 musl src/stdio/stderr.c
//! 标准错误输出流全局变量

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 内部缓冲区（UNGET 预留空间）
static mut BUF: [u8; UNGET] = [0; UNGET];

/// 标准错误输出 FILE 对象
/// fd=2, 无缓冲模式(buf_size=0), 永久+只写, lbf=EOF
#[no_mangle]
pub(crate) static mut __stderr_FILE: FILE = FILE {
    flags: F_PERM | F_NORD,
    rpos: core::ptr::null_mut(),
    rend: core::ptr::null_mut(),
    close: None, // TODO: Some(__stdio_close)
    wend: core::ptr::null_mut(),
    wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(),
    wbase: core::ptr::null_mut(),
    read: None,
    write: None, // TODO: Some(__stdio_write)
    seek: None,  // TODO: Some(__stdio_seek)
    buf: unsafe { core::ptr::addr_of_mut!(BUF).cast::<u8>().add(UNGET) },
    buf_size: 0,
    prev: core::ptr::null_mut(),
    next: core::ptr::null_mut(),
    fd: 2,
    pipe_pid: 0,
    lockcount: 0,
    mode: 0,
    lock: -1,
    lbf: EOF,
    cookie: core::ptr::null_mut(),
    off: 0,
    getln_buf: core::ptr::null_mut(),
    mustbezero_2: core::ptr::null_mut(),
    shend: core::ptr::null_mut(),
    shlim: 0,
    shcnt: 0,
    prev_locked: core::ptr::null_mut(),
    next_locked: core::ptr::null_mut(),
    locale: core::ptr::null_mut(),
};

/// stderr — 标准错误输出流, 指向 __stderr_FILE。
#[no_mangle]
pub static mut stderr: *mut FILE = unsafe { core::ptr::addr_of_mut!(__stderr_FILE) };

/// 内部哨兵变量，用于 __stdio_exit 退出刷新
#[no_mangle]
pub(crate) static mut __stderr_used: *mut FILE =
    unsafe { core::ptr::addr_of_mut!(__stderr_FILE) };
