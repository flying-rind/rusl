//! __fdopen — 从已打开的文件描述符构造 FILE 流对象。
//! 对应 musl src/stdio/__fdopen.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

// BUFSIZ typical value = 1024
const BUFSIZ: usize = 1024;

extern "C" {
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn free(ptr: *mut core::ffi::c_void);
}

fn c_malloc(size: usize) -> *mut u8 {
    unsafe { malloc(size) as *mut u8 }
}

fn c_free(ptr: *mut u8) {
    if !ptr.is_null() {
        unsafe { free(ptr as *mut core::ffi::c_void); }
    }
}

/// 检查 mode 首字符是否为有效值 ('r', 'w', 'a')
unsafe fn check_mode(mode: *const c_char) -> bool {
    let first = *mode;
    first == b'r' as c_char || first == b'w' as c_char || first == b'a' as c_char
}

/// 检查 mode 字符串是否包含字符 ch
unsafe fn strchr_mode(mode: *const c_char, ch: u8) -> bool {
    let mut i = 0;
    loop {
        let c = *mode.add(i);
        if c == 0 { break; }
        if c == ch as c_char { return true; }
        i += 1;
    }
    false
}

/// __fdopen — 主实现。从 fd 和 mode 字符串构造 FILE，分配内存、配置缓冲区、设置操作指针。
/// 分配 sizeof(FILE) + UNGET + BUFSIZ 字节，将流登记到全局打开文件链表。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fdopen(
    fd: c_int,
    mode: *const c_char,
) -> *mut FILE {
    unsafe {
        if !check_mode(mode) {
            return core::ptr::null_mut();
        }

        // 分配 FILE + UNGET + BUFSIZ
        let total_size = core::mem::size_of::<FILE>() + UNGET + BUFSIZ;
        let ptr = c_malloc(total_size) as *mut FILE;
        if ptr.is_null() {
            return core::ptr::null_mut();
        }

        // 清零 FILE 结构体
        core::ptr::write_bytes(ptr as *mut u8, 0, core::mem::size_of::<FILE>());

        let f = &mut *ptr;

        // 施加模式限制
        if !strchr_mode(mode, b'+') {
            f.flags = if *mode == b'r' as c_char { F_NOWR } else { F_NORD };
        }

        // 设置 append 模式
        if *mode == b'a' as c_char {
            f.flags |= F_APP;
        }

        f.fd = fd;
        // buf 指向结构体之后 + UNGET
        let buf_start = (ptr as *mut u8).add(core::mem::size_of::<FILE>() + UNGET);
        f.buf = buf_start;
        f.buf_size = BUFSIZ;

        // 默认为全缓冲
        f.lbf = super::stdio_impl::EOF;

        // 设置操作指针
        f.read = Some(super::__stdio_read::__stdio_read);
        f.write = Some(super::__stdio_write::__stdio_write);
        f.seek = Some(super::__stdio_seek::__stdio_seek);
        f.close = Some(super::__stdio_close::__stdio_close);

        // 单线程：无需锁
        f.lock = -1;

        // 添加到打开文件列表
        super::ofl_add::__ofl_add(ptr)
    }
}

/// fdopen — __fdopen 的弱别名。对外导出，行为与 __fdopen 完全一致。
#[no_mangle]
pub extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE {
    unsafe { __fdopen(fd, mode) }
}
