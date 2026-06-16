//! # 线程取消 API 桩
//!
//! POSIX 线程取消的发送、测试和状态/类型设置操作。

use core::ffi::c_int;

use crate::types::*;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_cancel(t: pthread_t) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_testcancel() {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {
    unimplemented!()
}

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int {
    unimplemented!()
}
