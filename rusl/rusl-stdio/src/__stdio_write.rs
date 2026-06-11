//! 对应 musl src/stdio/__stdio_write.c
//! 内部 FILE 默认写操作实现

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 通过 writev 系统调用将内部缓冲区和用户数据一并写入文件描述符
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdio_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unimplemented!()
}
