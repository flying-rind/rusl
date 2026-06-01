//! errno模块对外api

use core::ffi::{c_char, c_int};
use rusl_core::c_types::locale_t;

// ---------- internal FFI declarations ----------

extern "C" {
    /// musl libc errno 位置访问器。
    /// 返回当前线程 errno 变量的地址。
    #[link_name = "__errno_location"]
    fn musl___errno_location() -> *mut c_int;

    /// GNU 兼容弱别名, 行为与 __errno_location 完全一致。
    #[link_name = "___errno_location"]
    fn musl____errno_location() -> *mut c_int;

    /// 将 errno 错误码转换为可读错误描述字符串。
    #[link_name = "strerror"]
    fn musl_strerror(e: c_int) -> *mut c_char;

    /// locale 版本的 strerror (Stage 0 忽略 locale 参数)。
    #[link_name = "strerror_l"]
    fn musl_strerror_l(e: c_int, loc: locale_t) -> *mut c_char;
}

// ---------- public wrappers ----------

/// 返回当前线程 errno 变量的地址。
pub extern "C" fn __errno_location() -> *mut c_int {
    unsafe { musl___errno_location() }
}

/// GNU 兼容的 errno 位置访问器弱别名, 行为与 [`__errno_location`] 完全一致。
pub extern "C" fn ___errno_location() -> *mut c_int {
    unsafe { musl____errno_location() }
}

/// 返回 errno 错误码 `e` 对应的可读错误描述字符串。
///
/// 返回的指针指向静态只读数据, 调用者不应修改或释放。
pub extern "C" fn strerror(e: c_int) -> *mut c_char {
    unsafe { musl_strerror(e) }
}

/// 返回 errno 错误码 `e` 在指定 locale 下的可读错误描述字符串。
///
/// Stage 0: `loc` 参数被忽略, 行为与 `strerror` 完全一致。
///
/// 返回的指针指向静态只读数据, 调用者不应修改或释放。
pub extern "C" fn strerror_l(e: c_int, _loc: locale_t) -> *mut c_char {
    unsafe { musl_strerror_l(e, _loc) }
}