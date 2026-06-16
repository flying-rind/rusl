//! # 自旋锁 (pthread_spin) API 桩
//!
//! POSIX 自旋锁的初始化、销毁、加锁、尝试加锁和解锁操作。

use core::ffi::c_int;

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_spin_init(s: *mut pthread_spinlock_t, pshared: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_spin_destroy(s: *mut pthread_spinlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_spin_lock(s: *mut pthread_spinlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_spin_trylock(s: *mut pthread_spinlock_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_spin_unlock(s: *mut pthread_spinlock_t) -> c_int {
    unimplemented!()
}
