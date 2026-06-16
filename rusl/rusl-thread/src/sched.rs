//! # 线程调度 API 桩
//!
//! POSIX 线程调度策略、优先级和并发级别操作。

use core::ffi::c_int;

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getschedparam(t: pthread_t, policy: *mut c_int, param: *mut sched_param) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setschedparam(t: pthread_t, policy: c_int, param: *const sched_param) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setschedprio(t: pthread_t, prio: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getconcurrency() -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setconcurrency(val: c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_getcpuclockid(t: pthread_t, clk: *mut clockid_t) -> c_int {
    unimplemented!()
}
