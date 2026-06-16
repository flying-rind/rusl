//! # 线程信号 API 桩
//!
//! POSIX 线程信号发送和信号掩码操作。

use core::ffi::c_int;

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_kill(t: pthread_t, sig: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_sigmask(how: c_int, set: *const sigset_t, old: *mut sigset_t) -> c_int {
    unimplemented!()
}
