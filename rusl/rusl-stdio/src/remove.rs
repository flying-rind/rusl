//! 对应 musl src/stdio/remove.c
//! 删除指定路径的文件或空目录

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 从文件系统中删除 path 指向的文件或空目录
#[no_mangle]
pub extern "C" fn remove(path: *const c_char) -> c_int {
    unimplemented!()
}
