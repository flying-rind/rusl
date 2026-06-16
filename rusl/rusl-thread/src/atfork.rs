//! # Fork 处理 API 桩
//!
//! POSIX fork 前/后回调注册操作。

use core::ffi::c_int;

/// [Visibility]: User
#[no_mangle]
pub extern "C" fn pthread_atfork(
    prepare: Option<unsafe extern "C" fn()>,
    parent: Option<unsafe extern "C" fn()>,
    child: Option<unsafe extern "C" fn()>,
) -> c_int {
    unimplemented!()
}
