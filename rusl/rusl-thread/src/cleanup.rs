//! # 清理处理 API 桩
//!
//! POSIX 线程清理处理器的底层 push/pop 操作。
//! pthread_cleanup_push/pop 在 C 中为宏，rusl 导出底层函数。

use core::ffi::{c_int, c_void};

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn _pthread_cleanup_push(
    cb: *mut c_void,
    f: Option<unsafe extern "C" fn(*mut c_void)>,
    x: *mut c_void,
) {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn _pthread_cleanup_pop(cb: *mut c_void, execute: c_int) {
    unimplemented!()
}
