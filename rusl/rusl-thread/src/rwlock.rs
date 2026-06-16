//! # 读写锁 (pthread_rwlock) API 桩
//!
//! POSIX 读写锁的创建、销毁、读锁、写锁、解锁等操作。

use core::ffi::c_int;

use crate::types::*;

// ============================================================================
// 3.1 读写锁属性
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlockattr_init(a: *mut pthread_rwlockattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlockattr_destroy(a: *mut pthread_rwlockattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlockattr_setpshared(a: *mut pthread_rwlockattr_t, pshared: c_int) -> c_int {
    unimplemented!()
}

// ============================================================================
// 3.2 读写锁操作
// ============================================================================

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, a: *const pthread_rwlockattr_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_destroy(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, at: *const timespec) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int {
    unimplemented!()
}
