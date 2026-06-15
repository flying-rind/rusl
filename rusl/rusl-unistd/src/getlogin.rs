//! getlogin — 获取当前登录用户名（非线程安全）。
//! 对应 musl src/unistd/getlogin.c
//!
//! 查询环境变量 LOGNAME。

use core::ffi::c_char;

/// POSIX `getlogin` — 返回当前登录用户的名字。
///
/// musl 的实现直接查询环境变量 `LOGNAME`。
/// 返回的指针指向进程环境变量表，后续 `setenv`/`putenv` 可能使其失效。
/// 失败返回 NULL。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getlogin() -> *mut c_char {
    // 查询环境变量 LOGNAME
    extern "C" {
        fn getenv(name: *const c_char) -> *mut c_char;
    }
    unsafe { getenv(b"LOGNAME\0".as_ptr() as *const c_char) }
}
