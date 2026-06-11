//! 对应 musl src/stdio/stdin.c
//! 标准输入流全局变量

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 内部缓冲区（BUFSIZ + UNGET 预留空间）
const BUF_SIZE: usize = 1024 + UNGET;
static mut BUF: [u8; BUF_SIZE] = [0; BUF_SIZE];

/// 标准输入 FILE 对象
/// fd=0, 全缓冲模式(buf_size=1024), 永久+不可写, lbf=0
#[no_mangle]
pub(crate) static mut __stdin_FILE: FILE = FILE {
    flags: F_PERM | F_NOWR,
    rpos: core::ptr::null_mut(),
    rend: core::ptr::null_mut(),
    close: None, // TODO: Some(__stdio_close)
    wend: core::ptr::null_mut(),
    wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(),
    wbase: core::ptr::null_mut(),
    read: None,  // TODO: Some(__stdio_read)
    write: None,
    seek: None,  // TODO: Some(__stdio_seek)
    buf: unsafe { core::ptr::addr_of_mut!(BUF).cast::<u8>().add(UNGET) },
    buf_size: 1024,
    prev: core::ptr::null_mut(),
    next: core::ptr::null_mut(),
    fd: 0,
    pipe_pid: 0,
    lockcount: 0,
    mode: 0,
    lock: -1,
    lbf: 0,
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

/// stdin — 标准输入流, 指向 __stdin_FILE。
#[no_mangle]
pub static mut stdin: *mut FILE = core::ptr::addr_of_mut!(__stdin_FILE);

/// 内部哨兵变量，用于 __stdio_exit 退出刷新
#[no_mangle]
pub(crate) static mut __stdin_used: *mut FILE =
    core::ptr::addr_of_mut!(__stdin_FILE);
