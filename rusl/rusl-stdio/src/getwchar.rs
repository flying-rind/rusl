//! getwchar — 从标准输入 stdin 读取一个宽字符。
//! 对应 musl src/stdio/getwchar.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_uint;

/// 从标准输入流 stdin 读取一个宽字符。等价于 fgetwc(stdin) / getwc(stdin)。
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn getwchar() -> c_uint /* wint_t */ {
    unimplemented!()
}

/// 免锁版本（弱别名 -> getwchar）。
/// [Visibility]: User — POSIX 免锁扩展。
#[no_mangle]
pub extern "C" fn getwchar_unlocked() -> c_uint /* wint_t */ {
    unimplemented!()
}
