//! fgetpos — 获取文件流的当前逻辑位置。
//! 对应 musl src/stdio/fgetpos.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// fpos_t 在 musl 中等价于 c_longlong（i64）。
pub type fpos_t = i64;

/// fgetpos — 获取文件流当前逻辑位置，存入 *pos。
/// 内部调用 __ftello(f) 获取 off_t 位置。
/// 成功返回 0，失败返回 -1 且 *pos 不被修改。
#[no_mangle]
pub extern "C" fn fgetpos(_f: *mut FILE, _pos: *mut fpos_t) -> c_int {
    unimplemented!()
}
