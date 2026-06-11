//! 对应 musl src/stdio/pclose.c
//! 关闭 popen 打开的流，等待子进程退出并返回其状态

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 关闭 popen 打开的流，等待子进程退出，返回退出状态码
#[no_mangle]
pub extern "C" fn pclose(f: *mut FILE) -> c_int {
    unimplemented!()
}
