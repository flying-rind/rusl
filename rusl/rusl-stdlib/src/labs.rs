//! labs/llabs/imaxabs —— 计算 long/long long/intmax_t 的绝对值。对外导出 C ABI 兼容的符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;

/// 计算 `a` 的绝对值（`i64` 版本）。
///
/// # Safety
///
/// - `a` 可以为任意 `i64` 值，但当 `a == i64::MIN` 时行为未定义（无法用 `i64` 表示）。
///
/// # 返回值
///
/// - `a >= 0` 时返回 `a`。
/// - `a < 0` 时返回 `-a`。
#[no_mangle]
pub unsafe extern "C" fn labs(a: i64) -> i64 {
    if a > 0 { a } else { -a }
}

/// 计算 `a` 的绝对值（`i64` 版本）。
///
/// # Safety
///
/// - `a` 可以为任意 `i64` 值，但当 `a == i64::MIN` 时行为未定义（无法用 `i64` 表示）。
///
/// # 返回值
///
/// - `a >= 0` 时返回 `a`。
/// - `a < 0` 时返回 `-a`。
#[no_mangle]
pub unsafe extern "C" fn llabs(a: i64) -> i64 {
    if a > 0 { a } else { -a }
}

/// 计算 `a` 的绝对值（`intmax_t`/`i64` 版本）。
///
/// # Safety
///
/// - `a` 可以为任意 `i64` 值，但当 `a == i64::MIN` 时行为未定义（无法用 `i64` 表示）。
///
/// # 返回值
///
/// - `a >= 0` 时返回 `a`。
/// - `a < 0` 时返回 `-a`。
#[no_mangle]
pub unsafe extern "C" fn imaxabs(a: i64) -> i64 {
    if a > 0 { a } else { -a }
}
