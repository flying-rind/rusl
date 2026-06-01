//! atof —— 将字符串转换为 double。对外导出 C ABI 兼容的 `atof` 符号。
//!
//! 实现直接调用内部 `strtod` 函数（纯 Rust，无 FFI）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 将 `s` 指向的以 null 结尾的字符串转换为 `f64`。
///
/// 该函数等价于 `strtod(s, null)`，是 `strtod` 的薄封装。
///
/// # Safety
///
/// - `s` 必须指向以 null 结尾的有效 C 字符串。
///
/// # 返回值
///
/// - 成功解析：返回对应的 `f64` 值。
/// - 无有效数字：返回 `0.0`。
/// - 溢出：返回 `±HUGE_VAL`。
#[no_mangle]
pub unsafe extern "C" fn atof(s: *const c_char) -> f64 {
    super::strtod::strtod(s, core::ptr::null_mut())
}
