//! 对应 musl src/stdio/putwchar.c
//! 标准输出宽字符写入函数

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

/// 将宽字符 c 写入 stdout，等价于 fputwc(c, stdout)
#[no_mangle]
pub extern "C" fn putwchar(c: c_int) -> c_int {
    let f = unsafe { super::stdout::stdout };
    super::fputwc::fputwc(c, f)
}

/// putwchar_unlocked — putwchar 的弱别名
#[no_mangle]
pub extern "C" fn putwchar_unlocked(c: c_int) -> c_int {
    let f = unsafe { super::stdout::stdout };
    super::fputwc::fputwc_unlocked(c, f)
}
