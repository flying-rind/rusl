//! 对应 musl src/stdio/open_memstream.c
//! 创建动态内存流，自动增长的只写流

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;

/// 创建动态内存流。
/// - bufp: 输出参数，关闭时写入最终缓冲区地址
/// - sizep: 输出参数，实时更新缓冲区大小（不含 NULL 终止符）
/// 返回新创建的只写 FILE 指针，失败返回 NULL
#[no_mangle]
pub extern "C" fn open_memstream(
    bufp: *mut *mut u8,
    sizep: *mut usize,
) -> *mut FILE {
    unimplemented!()
}
