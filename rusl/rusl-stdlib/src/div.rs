//! div/ldiv/lldiv/imaxdiv —— 计算商和余数。对外导出 C ABI 兼容的符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;

/// div_t 结构体，包含 int 类型的商和余数。
#[repr(C)]
pub struct div_t {
    pub quot: i32,
    pub rem: i32,
}

/// ldiv_t 结构体，包含 long 类型的商和余数。
#[repr(C)]
pub struct ldiv_t {
    pub quot: i64,
    pub rem: i64,
}

/// lldiv_t 结构体，包含 long long 类型的商和余数。
#[repr(C)]
pub struct lldiv_t {
    pub quot: i64,
    pub rem: i64,
}

/// imaxdiv_t 结构体，包含 intmax_t 类型的商和余数。
#[repr(C)]
pub struct imaxdiv_t {
    pub quot: i64,
    pub rem: i64,
}

/// 计算 `num / den` 的商和余数（向零截断）。
///
/// # Safety
///
/// - `den` 必须非零（否则行为未定义）。
/// - `num == i32::MIN && den == -1` 时行为未定义（商溢出 `i32`）。
///
/// # 返回值
///
/// 返回 `div_t`，满足 `num == quot * den + rem`。
#[no_mangle]
pub extern "C" fn div(num: i32, den: i32) -> div_t {
    div_t {
        quot: num / den,
        rem: num % den,
    }
}

/// 计算 `num / den` 的商和余数（向零截断），类型为 `i64`。
///
/// # Safety
///
/// - `den` 必须非零（否则行为未定义）。
/// - `num == i64::MIN && den == -1` 时行为未定义（商溢出 `i64`）。
///
/// # 返回值
///
/// 返回 `ldiv_t`，满足 `num == quot * den + rem`。
#[no_mangle]
pub extern "C" fn ldiv(num: i64, den: i64) -> ldiv_t {
    ldiv_t {
        quot: num / den,
        rem: num % den,
    }
}

/// 计算 `num / den` 的商和余数（向零截断），类型为 `i64`。
///
/// # Safety
///
/// - `den` 必须非零（否则行为未定义）。
/// - `num == i64::MIN && den == -1` 时行为未定义（商溢出 `i64`）。
///
/// # 返回值
///
/// 返回 `lldiv_t`，满足 `num == quot * den + rem`。
#[no_mangle]
pub extern "C" fn lldiv(num: i64, den: i64) -> lldiv_t {
    lldiv_t {
        quot: num / den,
        rem: num % den,
    }
}

/// 计算 `num / den` 的商和余数（向零截断），类型为 `i64`。
///
/// # Safety
///
/// - `den` 必须非零（否则行为未定义）。
/// - `num == i64::MIN && den == -1` 时行为未定义（商溢出 `i64`）。
///
/// # 返回值
///
/// 返回 `imaxdiv_t`，满足 `num == quot * den + rem`。
#[no_mangle]
pub extern "C" fn imaxdiv(num: i64, den: i64) -> imaxdiv_t {
    imaxdiv_t {
        quot: num / den,
        rem: num % den,
    }
}
