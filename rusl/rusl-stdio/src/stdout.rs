//! 对应 musl src/stdio/stdout.c
//! 标准输出流全局变量

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 内部缓冲区（BUFSIZ + UNGET 预留空间）
const BUF_SIZE: usize = 1024 + UNGET;
static mut BUF: [u8; BUF_SIZE] = [0; BUF_SIZE];

/// 标准输出 FILE 对象
/// fd=1, 行缓冲模式(buf_size=1024, lbf=b'\n'), 永久+只写
#[no_mangle]
pub(crate) static mut __stdout_FILE: FILE = FILE {
    flags: F_PERM | F_NORD,
    rpos: core::ptr::null_mut(),
    rend: core::ptr::null_mut(),
    close: Some(super::__stdio_close::__stdio_close),
    wend: core::ptr::null_mut(),
    wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(),
    wbase: core::ptr::null_mut(),
    read: None,
    write: Some(super::__stdout_write::__stdout_write),
    seek: Some(super::__stdio_seek::__stdio_seek),
    buf: unsafe { core::ptr::addr_of_mut!(BUF).cast::<u8>().add(UNGET) },
    buf_size: 1024,
    prev: core::ptr::null_mut(),
    next: core::ptr::null_mut(),
    fd: 1,
    pipe_pid: 0,
    lockcount: 0,
    mode: 0,
    lock: -1,
    lbf: b'\n' as i32,
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

/// stdout — 标准输出流, 指向 __stdout_FILE。
#[no_mangle]
pub static mut stdout: *mut FILE = core::ptr::addr_of_mut!(__stdout_FILE);

/// 内部哨兵变量，用于 __stdio_exit 退出刷新
#[no_mangle]
pub(crate) static mut __stdout_used: *mut FILE =
    core::ptr::addr_of_mut!(__stdout_FILE);
