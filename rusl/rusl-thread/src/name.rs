//! # 线程命名 API 桩 (GNU 扩展)
//!
//! 获取和设置线程名称的 GNU 扩展操作。

use core::ffi::{c_char, c_int};

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setname_np(thread: pthread_t, name: *const c_char) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getname_np(thread: pthread_t, name: *mut c_char, len: usize) -> c_int {
    unimplemented!()
}
