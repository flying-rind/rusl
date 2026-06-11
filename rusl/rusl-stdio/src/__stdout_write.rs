//! 对应 musl src/stdio/__stdout_write.c
//! stdout 专用写函数 —— 首次写入时完成延迟初始化

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// stdout 延迟初始化写函数，首次调用后替换为 __stdio_write
#[no_mangle]
pub(crate) unsafe extern "C" fn __stdout_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    unimplemented!()
}
