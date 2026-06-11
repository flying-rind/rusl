//! 对应 musl src/stdio/putchar.c
//! 标准输出单字符写入实现

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

/// 将字符 c 写入 stdout，等价于 putc(c, stdout)
#[no_mangle]
pub extern "C" fn putchar(c: c_int) -> c_int {
    unimplemented!()
}
