//! vsscanf — 字符串格式化输入（va_list 版本）。
//! 对应 musl src/stdio/vsscanf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_void};

/// str_read 回调 — 从字符串读取数据到 FILE 缓冲区。
unsafe extern "C" fn str_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    unsafe {
        let src = (*f).cookie as *const u8;

        // 找到字符串末尾或最多读取 len 字节
        let mut k = 0usize;
        while k < len && *src.add(k) != 0 {
            k += 1;
        }

        let read_len = k;
        if read_len > 0 {
            core::ptr::copy_nonoverlapping(src, buf, read_len);
        }

        // 更新 cookie 指向已读位置之后
        (*f).cookie = src.add(read_len) as *mut c_void;

        read_len
    }
}

/// vsscanf — 从内存中的 null 结尾字符串读取格式化输入。
///
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe {
        if s.is_null() {
            return -1;
        }

        // 构建栈上的 FILE，用 str_read 作为读取回调
        let mut f: FILE = core::mem::zeroed();
        f.buf = s as *mut u8;
        f.cookie = s as *mut c_void;
        f.read = Some(str_read);
        f.lock = -1;
        f.buf_size = 0;

        super::vfscanf::vfscanf(&mut f as *mut FILE, fmt, ap)
    }
}

/// __isoc99_vsscanf — vsscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vsscanf(s: *const c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    vsscanf(s, fmt, ap)
}
