//! getchar — 从标准输入 stdin 读取一个字符。
//! 对应 musl src/stdio/getchar.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;

/// 从标准输入流 stdin 读取一个字符。等价于 getc(stdin) / fgetc(stdin)。
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn getchar() -> c_int {
    unimplemented!()
}
