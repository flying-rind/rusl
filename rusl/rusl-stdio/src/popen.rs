//! 对应 musl src/stdio/popen.c
//! 启动子进程执行 shell 命令，返回 FILE 流以读写其标准输入/输出

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_char;

/// 创建管道、fork 子进程执行 /bin/sh -c <cmd>，返回 FILE 流
#[no_mangle]
pub extern "C" fn popen(_cmd: *const c_char, _mode: *const c_char) -> *mut FILE {
    core::ptr::null_mut()
}
