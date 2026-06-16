//! # 条件变量 (pthread_cond) API 桩
//!
//! POSIX 条件变量的创建、销毁、等待、信号和广播操作。

use core::ffi::c_int;

use crate::types::*;

// ============================================================================
// 4.1 条件变量属性
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_init(a: *mut pthread_condattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_destroy(a: *mut pthread_condattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_setclock(a: *mut pthread_condattr_t, clk: clockid_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_setpshared(a: *mut pthread_condattr_t, pshared: c_int) -> c_int {
    unimplemented!()
}

// ============================================================================
// 4.2 条件变量操作
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_init(c: *mut pthread_cond_t, a: *const pthread_condattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_destroy(c: *mut pthread_cond_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_wait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_timedwait(c: *mut pthread_cond_t, m: *mut pthread_mutex_t, ts: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_signal(c: *mut pthread_cond_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cond_broadcast(c: *mut pthread_cond_t) -> c_int {
    unimplemented!()
}
