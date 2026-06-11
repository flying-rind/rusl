//! 对应 musl src/stdio/perror.c
//! 向 stderr 输出与当前 errno 对应的错误消息

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 向 stderr 输出 `<msg>: <error_message>\n` 格式的错误信息
#[no_mangle]
pub extern "C" fn perror(msg: *const c_char) {
    unimplemented!()
}
