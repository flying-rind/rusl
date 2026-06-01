//! Stdlib — 标准库工具函数 (数值转换、排序、搜索、算术)

use core::ffi::{c_char, c_int, c_void};
pub use rusl_core::c_types::wchar_t;

// ---------- types ----------

#[repr(C)]
pub struct div_t {
    pub quot: i32,
    pub rem: i32,
}

#[repr(C)]
pub struct ldiv_t {
    pub quot: i64,
    pub rem: i64,
}

#[repr(C)]
pub struct lldiv_t {
    pub quot: i64,
    pub rem: i64,
}

#[repr(C)]
pub struct imaxdiv_t {
    pub quot: i64,
    pub rem: i64,
}

pub type CmpFun = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;
pub type CmpFunR = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> i32;

// ---------- internal FFI declarations ----------

extern "C" {
    #[link_name = "abs"]
    fn musl_abs(a: c_int) -> c_int;
    #[link_name = "atof"]
    fn musl_atof(s: *const c_char) -> f64;
    #[link_name = "atoi"]
    fn musl_atoi(s: *const c_char) -> i32;
    #[link_name = "atol"]
    fn musl_atol(s: *const c_char) -> i64;
    #[link_name = "atoll"]
    fn musl_atoll(s: *const c_char) -> i64;
    #[link_name = "bsearch"]
    fn musl_bsearch(key: *const c_void, base: *const c_void, nel: usize, width: usize, cmp: Option<CmpFun>) -> *mut c_void;
    #[link_name = "div"]
    fn musl_div(num: i32, den: i32) -> div_t;
    #[link_name = "ecvt"]
    fn musl_ecvt(x: f64, n: c_int, dp: *mut c_int, sign: *mut c_int) -> *mut c_char;
    #[link_name = "fcvt"]
    fn musl_fcvt(x: f64, n: c_int, dp: *mut c_int, sign: *mut c_int) -> *mut c_char;
    #[link_name = "gcvt"]
    fn musl_gcvt(x: f64, n: c_int, b: *mut c_char) -> *mut c_char;
    #[link_name = "imaxabs"]
    fn musl_imaxabs(a: i64) -> i64;
    #[link_name = "imaxdiv"]
    fn musl_imaxdiv(num: i64, den: i64) -> imaxdiv_t;
    #[link_name = "labs"]
    fn musl_labs(a: i64) -> i64;
    #[link_name = "ldiv"]
    fn musl_ldiv(num: i64, den: i64) -> ldiv_t;
    #[link_name = "lldiv"]
    fn musl_lldiv(num: i64, den: i64) -> lldiv_t;
    #[link_name = "llabs"]
    fn musl_llabs(a: i64) -> i64;
    #[link_name = "qsort"]
    fn musl_qsort(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFun>);
    #[link_name = "qsort_r"]
    fn musl_qsort_r(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFunR>, arg: *mut c_void);
    #[link_name = "strtod"]
    fn musl_strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;
    #[link_name = "strtof"]
    fn musl_strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32;
    #[link_name = "strtold"]
    fn musl_strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64;
    #[link_name = "strtol"]
    fn musl_strtol(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    #[link_name = "strtoll"]
    fn musl_strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    #[link_name = "strtoul"]
    fn musl_strtoul(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64;
    #[link_name = "strtoull"]
    fn musl_strtoull(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64;
    #[link_name = "strtoimax"]
    fn musl_strtoimax(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    #[link_name = "strtoumax"]
    fn musl_strtoumax(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64;
    #[link_name = "wcstod"]
    fn musl_wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64;
    #[link_name = "wcstof"]
    fn musl_wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32;
    #[link_name = "wcstold"]
    fn musl_wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64;
    #[link_name = "wcstol"]
    fn musl_wcstol(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64;
    #[link_name = "wcstoll"]
    fn musl_wcstoll(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64;
    #[link_name = "wcstoul"]
    fn musl_wcstoul(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64;
    #[link_name = "wcstoull"]
    fn musl_wcstoull(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64;
    #[link_name = "wcstoimax"]
    fn musl_wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64;
    #[link_name = "wcstoumax"]
    fn musl_wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64;
}

// ---------- safe public wrappers ----------

pub extern "C" fn abs(a: c_int) -> c_int                                            { unsafe { musl_abs(a) } }
pub extern "C" fn atof(s: *const c_char) -> f64                                     { unsafe { musl_atof(s) } }
pub extern "C" fn atoi(s: *const c_char) -> i32                                     { unsafe { musl_atoi(s) } }
pub extern "C" fn atol(s: *const c_char) -> i64                                     { unsafe { musl_atol(s) } }
pub extern "C" fn atoll(s: *const c_char) -> i64                                    { unsafe { musl_atoll(s) } }
pub extern "C" fn bsearch(key: *const c_void, base: *const c_void, nel: usize, width: usize, cmp: Option<CmpFun>) -> *mut c_void { unsafe { musl_bsearch(key, base, nel, width, cmp) } }
pub extern "C" fn div(num: i32, den: i32) -> div_t                                  { unsafe { musl_div(num, den) } }
pub extern "C" fn ecvt(x: f64, n: c_int, dp: *mut c_int, sign: *mut c_int) -> *mut c_char { unsafe { musl_ecvt(x, n, dp, sign) } }
pub extern "C" fn fcvt(x: f64, n: c_int, dp: *mut c_int, sign: *mut c_int) -> *mut c_char { unsafe { musl_fcvt(x, n, dp, sign) } }
pub extern "C" fn gcvt(x: f64, n: c_int, b: *mut c_char) -> *mut c_char            { unsafe { musl_gcvt(x, n, b) } }
pub extern "C" fn imaxabs(a: i64) -> i64                                            { unsafe { musl_imaxabs(a) } }
pub extern "C" fn imaxdiv(num: i64, den: i64) -> imaxdiv_t                          { unsafe { musl_imaxdiv(num, den) } }
pub extern "C" fn labs(a: i64) -> i64                                               { unsafe { musl_labs(a) } }
pub extern "C" fn ldiv(num: i64, den: i64) -> ldiv_t                                { unsafe { musl_ldiv(num, den) } }
pub extern "C" fn lldiv(num: i64, den: i64) -> lldiv_t                              { unsafe { musl_lldiv(num, den) } }
pub extern "C" fn llabs(a: i64) -> i64                                              { unsafe { musl_llabs(a) } }
pub extern "C" fn qsort(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFun>) { unsafe { musl_qsort(base, nel, width, cmp) } }
pub extern "C" fn qsort_r(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFunR>, arg: *mut c_void) { unsafe { musl_qsort_r(base, nel, width, cmp, arg) } }
pub extern "C" fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64        { unsafe { musl_strtod(s, endptr) } }
pub extern "C" fn strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32        { unsafe { musl_strtof(s, endptr) } }
pub extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64       { unsafe { musl_strtold(s, endptr) } }
pub extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64 { unsafe { musl_strtol(s, endptr, base) } }
pub extern "C" fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64 { unsafe { musl_strtoll(s, endptr, base) } }
pub extern "C" fn strtoul(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64 { unsafe { musl_strtoul(s, endptr, base) } }
pub extern "C" fn strtoull(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64 { unsafe { musl_strtoull(s, endptr, base) } }
pub extern "C" fn strtoimax(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64 { unsafe { musl_strtoimax(s, endptr, base) } }
pub extern "C" fn strtoumax(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> u64 { unsafe { musl_strtoumax(s, endptr, base) } }
pub extern "C" fn wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64      { unsafe { musl_wcstod(s, endptr) } }
pub extern "C" fn wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32      { unsafe { musl_wcstof(s, endptr) } }
pub extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64     { unsafe { musl_wcstold(s, endptr) } }
pub extern "C" fn wcstol(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64 { unsafe { musl_wcstol(s, endptr, base) } }
pub extern "C" fn wcstoll(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64 { unsafe { musl_wcstoll(s, endptr, base) } }
pub extern "C" fn wcstoul(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64 { unsafe { musl_wcstoul(s, endptr, base) } }
pub extern "C" fn wcstoull(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64 { unsafe { musl_wcstoull(s, endptr, base) } }
pub extern "C" fn wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> i64 { unsafe { musl_wcstoimax(s, endptr, base) } }
pub extern "C" fn wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> u64 { unsafe { musl_wcstoumax(s, endptr, base) } }