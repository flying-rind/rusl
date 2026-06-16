//! # 屏障 (pthread_barrier) API 桩
//!
//! POSIX 屏障的创建、销毁和等待操作。

use core::ffi::{c_int, c_uint};

use crate::types::*;

// ============================================================================
// 5.1 屏障属性
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrierattr_init(a: *mut pthread_barrierattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrierattr_destroy(a: *mut pthread_barrierattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrierattr_setpshared(a: *mut pthread_barrierattr_t, pshared: c_int) -> c_int {
    unimplemented!()
}

// ============================================================================
// 5.2 屏障操作
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrier_init(b: *mut pthread_barrier_t, a: *const pthread_barrierattr_t, count: c_uint) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrier_destroy(b: *mut pthread_barrier_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int {
    unimplemented!()
}
