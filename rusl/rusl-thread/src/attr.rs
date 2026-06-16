//! # 线程属性 (pthread_attr) API 桩
//!
//! POSIX 线程属性对象的创建、销毁、获取和设置操作。
//! 还包括 barrier/cond/mutex/rwlock 的属性操作和 GNU np 扩展。

use core::ffi::{c_int, c_void};

use crate::types::*;

// ============================================================================
// 1.1 属性初始化与销毁
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_init(a: *mut pthread_attr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_destroy(a: *mut pthread_attr_t) -> c_int {
    unimplemented!()
}

// ============================================================================
// 1.2 属性获取 (Getter)
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getdetachstate(a: *const pthread_attr_t, state: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getguardsize(a: *const pthread_attr_t, size: *mut usize) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getinheritsched(a: *const pthread_attr_t, inherit: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getschedparam(a: *const pthread_attr_t, param: *mut sched_param) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getschedpolicy(a: *const pthread_attr_t, policy: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getscope(a: *const pthread_attr_t, scope: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getstack(a: *const pthread_attr_t, addr: *mut *mut c_void, size: *mut usize) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_getstacksize(a: *const pthread_attr_t, size: *mut usize) -> c_int {
    unimplemented!()
}

// ============================================================================
// 1.3 属性设置 (Setter)
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setdetachstate(a: *mut pthread_attr_t, state: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setguardsize(a: *mut pthread_attr_t, size: usize) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setinheritsched(a: *mut pthread_attr_t, inherit: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setschedparam(a: *mut pthread_attr_t, param: *const sched_param) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setschedpolicy(a: *mut pthread_attr_t, policy: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setscope(a: *mut pthread_attr_t, scope: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setstack(a: *mut pthread_attr_t, addr: *mut c_void, size: usize) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_attr_setstacksize(a: *mut pthread_attr_t, size: usize) -> c_int {
    unimplemented!()
}

// ============================================================================
// 1.4 其他属性类型获取
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_barrierattr_getpshared(a: *const pthread_barrierattr_t, pshared: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_getclock(a: *const pthread_condattr_t, clk: *mut clockid_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_condattr_getpshared(a: *const pthread_condattr_t, pshared: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_mutexattr_getprotocol(a: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_mutexattr_getpshared(a: *const pthread_mutexattr_t, pshared: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_mutexattr_getrobust(a: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_mutexattr_gettype(a: *const pthread_mutexattr_t, type_: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlockattr_getpshared(a: *const pthread_rwlockattr_t, pshared: *mut c_int) -> c_int {
    unimplemented!()
}

// ============================================================================
// 1.5 GNU 扩展 (np)
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getattr_default_np(attrp: *mut pthread_attr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setattr_default_np(attrp: *const pthread_attr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getattr_np(t: pthread_t, a: *mut pthread_attr_t) -> c_int {
    unimplemented!()
}
