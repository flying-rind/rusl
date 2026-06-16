//! # 一次性初始化 (pthread_once) API 桩
//!
//! POSIX 线程安全的一次性初始化操作。

use core::ffi::{c_int, c_void};

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_once(control: *mut pthread_once_t, init: Option<unsafe extern "C" fn()>) -> c_int {
    unimplemented!()
}
