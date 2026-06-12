//! fgets — 从 FILE 流中读取一行字符串到用户缓冲区。
//! 对应 musl src/stdio/fgets.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

/// 内部实现
unsafe fn __fgets_impl(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char {
    if n <= 1 {
        if n == 1 {
            *s = 0;
        }
        return if n <= 0 { core::ptr::null_mut() } else { s };
    }

    let mut i: c_int = 0;
    while i < n - 1 {
        let c = super::getc_unlocked::getc_unlocked(f);
        if c == EOF {
            break;
        }
        *s.add(i as usize) = c as c_char;
        i += 1;
        if c == b'\n' as c_int {
            break;
        }
    }
    *s.add(i as usize) = 0;

    if i == 0 {
        return core::ptr::null_mut();
    }
    s
}

/// fgets — 从 FILE 流中读取至多 n-1 个字符到 s，遇到 '\n' 或 EOF 时停止。
/// 读取的字符串以 '\0' 结尾（n >= 1 时）。换行符保留在缓冲区中。
/// 返回 s（成功）或 NULL（失败/EOF 且未读取任何字符）。
#[no_mangle]
pub extern "C" fn fgets(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char {
    unsafe { __fgets_impl(s, n, f) }
}

/// fgets_unlocked — fgets 的弱别名。行为与 fgets 完全一致。
#[no_mangle]
pub extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char {
    unsafe { __fgets_impl(s, n, f) }
}
