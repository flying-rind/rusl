//! vasprintf — 动态分配缓冲区的格式化输出（va_list 版本，GNU 扩展）。
//! 对应 musl src/stdio/vasprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

/// vasprintf — 动态分配缓冲区并格式化输出。
///
/// [Visibility]: User — <stdio.h> GNU 扩展函数。
#[no_mangle]
pub extern "C" fn vasprintf(s: *mut *mut c_char, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe {
        // Phase 1: 计算所需长度
        // vsnprintf(NULL, 0, fmt, ap) 返回若缓冲区足够大时本应写入的字符数
        let l = super::vsnprintf::vsnprintf(core::ptr::null_mut(), 0, fmt, ap);
        if l < 0 {
            return -1;
        }

        // Phase 2: 分配缓冲区 (+1 for '\0')
        let buf = malloc((l as usize) + 1);
        if buf.is_null() {
            return -1;
        }
        *s = buf as *mut c_char;

        // Phase 3: 写入格式化结果
        super::vsnprintf::vsnprintf(*s, (l as usize) + 1, fmt, ap)
    }
}
