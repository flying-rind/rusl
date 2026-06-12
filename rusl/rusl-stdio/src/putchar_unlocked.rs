//! 对应 musl src/stdio/putchar_unlocked.c
//! 免锁标准输出单字符写入

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

/// 将字符 c 写入 stdout，不获取流锁
#[no_mangle]
pub extern "C" fn putchar_unlocked(c: c_int) -> c_int {
    let f = unsafe { super::stdout::stdout };
    super::putc_unlocked::putc_unlocked(c, f)
}
