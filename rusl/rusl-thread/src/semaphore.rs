//! # 信号量 API 桩
//!
//! POSIX 匿名和有名信号量的创建、销毁、操作。

use core::ffi::{c_char, c_int, c_uint};

use crate::types::*;

// ============================================================================
// 17.1 匿名信号量
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_destroy(sem: *mut sem_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_getvalue(sem: *mut sem_t, valp: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_wait(sem: *mut sem_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_trywait(sem: *mut sem_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_timedwait(sem: *mut sem_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_post(sem: *mut sem_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 17.2 有名信号量
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub unsafe extern "C" fn sem_open(name: *const c_char, flags: c_int, _: ...) -> *mut sem_t {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_close(sem: *mut sem_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn sem_unlink(name: *const c_char) -> c_int {
    unimplemented!()
}
