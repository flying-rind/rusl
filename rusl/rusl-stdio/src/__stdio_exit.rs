//! 对应 musl src/stdio/__stdio_exit.c
//! 程序退出时的 stdio 清理函数

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 程序退出时刷新所有 stdio 流的入口函数
#[no_mangle]
pub unsafe extern "C" fn __stdio_exit() {
    unsafe {
        let s_out = super::stdout::stdout;
        let s_err = super::stderr::stderr;
        if !s_out.is_null() {
            super::fflush::fflush(s_out);
        }
        if !s_err.is_null() {
            super::fflush::fflush(s_err);
        }
    }
}

/// __stdio_exit 的弱别名，供 musl 退出路径引用链判断
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_exit_needed() {
    unsafe { __stdio_exit(); }
}
