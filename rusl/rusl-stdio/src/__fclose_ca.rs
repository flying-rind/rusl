//! __fclose_ca — 调用方分配的 FILE 关闭操作。
//! 对应 musl src/stdio/__fclose_ca.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// __fclose_ca — 关闭调用方分配的 FILE 流。
/// 仅调用 f->close(f) 关闭底层文件描述符，不释放 FILE 内存。
/// 与 __fopen_rb_ca 配套使用。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fclose_ca(_f: *mut FILE) -> c_int {
    unimplemented!()
}
