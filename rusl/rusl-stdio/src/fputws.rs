//! fputws — 将宽字符串转换为多字节序列并批量写入 FILE 流。
//! 对应 musl src/stdio/fputws.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use super::stdio_impl::FILE;

/// 将宽字符串 ws 转换为多字节序列并写入 FILE 流 f。
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn fputws(ws: *const c_int /* const wchar_t */, f: *mut FILE) -> c_int {
    unsafe {
        if ws.is_null() {
            return 0;
        }
        let mut i = 0;
        loop {
            let wc = *ws.add(i);
            if wc == 0 { break; }
            if super::putc_unlocked::putc_unlocked(wc, f) < 0 {
                return -1;
            }
            i += 1;
        }
        0
    }
}

/// 免锁版本（弱别名 -> fputws）。
#[no_mangle]
pub extern "C" fn fputws_unlocked(ws: *const c_int /* const wchar_t */, f: *mut FILE) -> c_int {
    fputws(ws, f)
}
