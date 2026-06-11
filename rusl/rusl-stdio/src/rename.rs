//! 对应 musl src/stdio/rename.c
//! 将文件或目录从旧路径重命名为新路径

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// 将文件系统对象从 old 路径重命名为 new 路径
#[no_mangle]
pub extern "C" fn rename(old: *const c_char, new: *const c_char) -> c_int {
    unimplemented!()
}
