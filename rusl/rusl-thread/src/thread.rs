//! # 线程生命周期 API 桩
//!
//! POSIX 线程的创建、退出、等待、分离、标识和比较操作。

use core::ffi::{c_int, c_void};

use crate::types::*;

// ============================================================================
// 8. 线程生命周期
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_create(
    t: *mut pthread_t,
    a: *const pthread_attr_t,
    f: Option<unsafe extern "C" fn(*mut c_void) -> *mut c_void>,
    arg: *mut c_void,
) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_exit(retval: *mut c_void) {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_join(t: pthread_t, res: *mut *mut c_void) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_detach(t: pthread_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 11. 线程标识
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_self() -> pthread_t {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_equal(a: pthread_t, b: pthread_t) -> c_int {
    unimplemented!()
}
