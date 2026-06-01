//! env模块对外api

use core::ffi::{c_char, c_int};

// ---------- 全局变量 ----------

extern "C" {
    /// POSIX 标准环境变量指针。
    /// 指向以空指针终止的 `"NAME=VALUE"` 格式字符串指针数组。
    #[link_name = "environ"]
    pub static mut environ: *mut *mut c_char;
}

// ---------- internal FFI declarations ----------

extern "C" {
    #[link_name = "getenv"]
    fn musl_getenv(name: *const c_char) -> *mut c_char;
    #[link_name = "setenv"]
    fn musl_setenv(var: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    #[link_name = "unsetenv"]
    fn musl_unsetenv(name: *const c_char) -> c_int;
    #[link_name = "putenv"]
    fn musl_putenv(s: *mut c_char) -> c_int;
    #[link_name = "clearenv"]
    fn musl_clearenv() -> c_int;
    #[link_name = "secure_getenv"]
    fn musl_secure_getenv(name: *const c_char) -> *mut c_char;
}

// ---------- safe public wrappers ----------

pub extern "C" fn getenv(name: *const c_char) -> *mut c_char {
    unsafe { musl_getenv(name) }
}

pub extern "C" fn setenv(var: *const c_char, value: *const c_char, overwrite: c_int) -> c_int {
    unsafe { musl_setenv(var, value, overwrite) }
}

pub extern "C" fn unsetenv(name: *const c_char) -> c_int {
    unsafe { musl_unsetenv(name) }
}

pub extern "C" fn putenv(s: *mut c_char) -> c_int {
    unsafe { musl_putenv(s) }
}

pub extern "C" fn clearenv() -> c_int {
    unsafe { musl_clearenv() }
}

pub extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char {
    unsafe { musl_secure_getenv(name) }
}