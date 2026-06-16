//! # 线程局部存储 (TSD) API 桩
//!
//! POSIX 线程局部存储键的创建、删除、获取和设置操作。

use core::ffi::{c_int, c_void};

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_key_create(
    k: *mut pthread_key_t,
    dtor: Option<unsafe extern "C" fn(*mut c_void)>,
) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_key_delete(k: pthread_key_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getspecific(k: pthread_key_t) -> *mut c_void {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setspecific(k: pthread_key_t, x: *const c_void) -> c_int {
    unimplemented!()
}
