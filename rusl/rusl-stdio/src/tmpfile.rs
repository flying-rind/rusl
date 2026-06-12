//! tmpfile — 创建临时文件，关闭或程序退出时自动删除。
//! 对应 musl src/stdio/tmpfile.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 创建临时文件，返回 FILE 指针。
#[no_mangle]
pub extern "C" fn tmpfile() -> *mut FILE {
    // 简化：创建 /tmp 下的临时文件
    let template = b"/tmp/tmp.XXXXXX\0";
    // TODO: 实现 mkstemp
    core::ptr::null_mut()
}
