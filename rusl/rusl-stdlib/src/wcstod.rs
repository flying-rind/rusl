//! wcstod/wcstof/wcstold —— 将宽字符串转换为浮点数。对外导出 C ABI 兼容的符号。
//!
//! 实现将宽字符串转换为窄字节字符串后委托给 strtod 族函数。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// C 语言 wchar_t 类型（在 Linux 上为 32 位整数）。
type wchar_t = i32;

/// 判断宽字符是否为空白（ASCII 范围）。
#[inline]
fn is_wspace(wc: i32) -> bool {
    matches!(wc as u8, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// 将宽字符转换为窄字节。非 ASCII 字符映射为 '@'（与 musl 行为一致）。
#[inline]
fn wc_to_byte(wc: i32) -> u8 {
    if wc >= 0 && wc < 128 {
        wc as u8
    } else {
        b'@'
    }
}

/// 内部：将宽字符串转换为窄字节缓冲区，然后委托 strtod 解析。
///
/// 返回 (解析值, 宽字符串中消耗的字符数)。
unsafe fn wcstox_inner(s: *const wchar_t, endptr: *mut *mut wchar_t) -> (f64, bool) {
    // 1. 跳过前导空白
    let mut t = s;
    while *t != 0 && is_wspace(*t) {
        t = t.offset(1);
    }

    // 2. 将剩余部分转换为窄字节缓冲区
    //    最大支持 2048 字节（足以处理大部分场景）
    let mut buf = [0u8; 2048];
    let mut i: usize = 0;
    let mut p = t;
    while *p != 0 && i < buf.len() - 1 {
        buf[i] = wc_to_byte(*p);
        i += 1;
        p = p.offset(1);
    }
    buf[i] = 0; // null 终止

    // 3. 调用 strtod 解析窄字节字符串
    let mut byte_end: *mut c_char = core::ptr::null_mut();
    let result = super::strtod::strtod(buf.as_ptr() as *const c_char, &mut byte_end);

    // 4. 计算宽字符串 endptr
    if !endptr.is_null() {
        if byte_end.is_null() || byte_end == buf.as_ptr() as *mut c_char {
            // 无有效转换
            *endptr = s as *mut wchar_t;
        } else {
            let bytes_consumed = (byte_end as usize).wrapping_sub(buf.as_ptr() as usize);
            // 字节消耗数即宽字符消耗数（一一对应）
            *endptr = t.add(bytes_consumed) as *mut wchar_t;
        }
    }

    (result, false)
}

// ---------- 公开 API ----------

/// 将 `s` 指向的宽字符串转换为 `f64`。
#[no_mangle]
pub unsafe extern "C" fn wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    let (val, _) = wcstox_inner(s, endptr);
    val
}

/// 将 `s` 指向的宽字符串转换为 `f32`。
#[no_mangle]
pub unsafe extern "C" fn wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32 {
    let (val, _) = wcstox_inner(s, endptr);
    val as f32
}

/// 将 `s` 指向的宽字符串转换为 `f64`（long double 精度）。
#[no_mangle]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    let (val, _) = wcstox_inner(s, endptr);
    val
}

// ---------- 测试 ----------
