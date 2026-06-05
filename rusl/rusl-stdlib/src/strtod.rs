//! strtod/strtof/strtold —— 将字符串转换为浮点数。对外导出 C ABI 兼容的符号。
//!
//! 纯 Rust 实现，通过字节扫描确定数字范围后委托 core::str::FromStr 解析。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::str::FromStr;
use rusl_core::errno::__errno_location;

/// Linux ERANGE 值
const ERANGE: i32 = 34;

/// 设置 errno = ERANGE（仅在非测试构建中有效）。
#[inline]
unsafe fn set_erange() {
    #[cfg(not(test))]
    unsafe {
        *__errno_location() = ERANGE;
    }
}

// ---------- 辅助：跳过空白 ----------

#[inline]
fn skip_ws(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

// ---------- 扫描数字结束位置 ----------

/// 从 `pos` 开始扫描，找到数字的结束位置（不含）。
/// 对于无效数字返回 `None`（即 pos 位置不像是数字的开头）。
fn scan_float_end(bytes: &[u8], start: usize) -> Option<usize> {
    let len = bytes.len();
    if start >= len {
        return None;
    }

    let mut pos = start;

    // 可选符号
    if pos < len && (bytes[pos] == b'-' || bytes[pos] == b'+') {
        pos += 1;
    }

    if pos >= len {
        return None;
    }

    // 检查 inf/infinity
    if pos + 2 < len {
        let c0 = bytes[pos].to_ascii_lowercase();
        let c1 = bytes[pos + 1].to_ascii_lowercase();
        let c2 = bytes[pos + 2].to_ascii_lowercase();
        if c0 == b'i' && c1 == b'n' && c2 == b'f' {
            if pos + 8 <= len {
                let inf8: [u8; 8] = [
                    bytes[pos].to_ascii_lowercase(),
                    bytes[pos + 1].to_ascii_lowercase(),
                    bytes[pos + 2].to_ascii_lowercase(),
                    bytes[pos + 3].to_ascii_lowercase(),
                    bytes[pos + 4].to_ascii_lowercase(),
                    bytes[pos + 5].to_ascii_lowercase(),
                    bytes[pos + 6].to_ascii_lowercase(),
                    bytes[pos + 7].to_ascii_lowercase(),
                ];
                if &inf8 == b"infinity" {
                    return Some(pos + 8);
                }
            }
            return Some(pos + 3);
        }
        if c0 == b'n' && c1 == b'a' && c2 == b'n' {
            let mut end = pos + 3;
            if end < len && bytes[end] == b'(' {
                end += 1;
                while end < len && bytes[end] != b')' {
                    end += 1;
                }
                if end < len {
                    end += 1; // 跳过 ')'
                }
            }
            return Some(end);
        }
    }

    // 检查十六进制浮点数 (0x / 0X)
    let mut is_hex = false;
    if pos + 1 < len && bytes[pos] == b'0' && (bytes[pos + 1] == b'x' || bytes[pos + 1] == b'X') {
        is_hex = true;
        pos += 2;
    }

    if pos >= len {
        return None;
    }

    // 解析整数部分的数字
    let mut had_digits = false;

    if is_hex {
        while pos < len && bytes[pos].is_ascii_hexdigit() {
            had_digits = true;
            pos += 1;
        }
    } else {
        while pos < len && bytes[pos].is_ascii_digit() {
            had_digits = true;
            pos += 1;
        }
    }

    // 可选小数点
    if pos < len && bytes[pos] == b'.' {
        pos += 1;
        if is_hex {
            while pos < len && bytes[pos].is_ascii_hexdigit() {
                had_digits = true;
                pos += 1;
            }
        } else {
            while pos < len && bytes[pos].is_ascii_digit() {
                had_digits = true;
                pos += 1;
            }
        }
    }

    if !had_digits {
        return None; // 没有有效数字
    }

    // 可选指数部分
    if pos < len {
        if is_hex && (bytes[pos] == b'p' || bytes[pos] == b'P') {
            pos += 1;
            if pos < len && (bytes[pos] == b'-' || bytes[pos] == b'+') {
                pos += 1;
            }
            // 指数必须至少有一位数字
            let exp_start = pos;
            while pos < len && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == exp_start {
                // hex float 缺少指数数字：回退到 p 之前的位置
                // 先回退符号位
                // 注意：pos 当前在 p/P 的后一位（符号位，如果有）
                // 如果指数无数字，整个 hex float 无效
                return None;
            }
        } else if !is_hex && (bytes[pos] == b'e' || bytes[pos] == b'E') {
            let exp_pos = pos;
            pos += 1;
            if pos < len && (bytes[pos] == b'-' || bytes[pos] == b'+') {
                pos += 1;
            }
            let exp_start = pos;
            while pos < len && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            if pos == exp_start {
                // e 后有符号但无数字：回退到 e 之前
                pos = exp_pos;
            }
        }
    }

    Some(pos)
}

// ---------- 十六进制浮点数解析 ----------

/// 解析十六进制浮点数字符串（如 "0x1.ffffp+10"），不含前导空白和符号。
///
/// 格式: 0xHEXDIGITS[.HEXDIGITS]p[+-]DECDIGITS
fn parse_hex_float(slice: &[u8]) -> Option<(f64, bool)> {
    if slice.len() < 2 || slice[0] != b'0' || (slice[1] != b'x' && slice[1] != b'X') {
        return None;
    }
    let s = unsafe { core::str::from_utf8_unchecked(slice) };
    let mut chars = s.chars();

    // 跳过 "0x"/"0X"
    chars.next();
    chars.next();

    // 解析整数部分（十六进制数字）
    let mut int_part: u64 = 0;
    let mut frac_part: u64 = 0;
    let mut frac_digits: u32 = 0;
    let mut after_dot = false;
    let mut mantissa_digits = 0;

    loop {
        match chars.clone().next() {
            Some(c) if c.is_ascii_hexdigit() => {
                chars.next();
                let d = c.to_digit(16).unwrap() as u64;
                mantissa_digits += 1;
                if after_dot {
                    if frac_digits < 16 {
                        frac_part = frac_part.wrapping_mul(16).wrapping_add(d);
                        frac_digits += 1;
                    }
                } else {
                    int_part = int_part.wrapping_mul(16).wrapping_add(d);
                }
            }
            Some('.') => {
                chars.next();
                after_dot = true;
            }
            Some('p') | Some('P') => {
                chars.next();
                break;
            }
            _ => {
                return None;
            }
        }
    }

    if mantissa_digits == 0 {
        return None;
    }

    // 解析十进制指数 (p 后面的部分)
    let exp_sign = match chars.clone().next() {
        Some('-') => { chars.next(); -1 }
        Some('+') => { chars.next(); 1 }
        _ => 1
    };

    // 解析十进制指数（手动解析，避免在 no_std 下需要 alloc）
    let mut exp_val: i32 = 0;
    for c in chars {
        if let Some(d) = c.to_digit(10) {
            exp_val = exp_val.saturating_mul(10).saturating_add(d as i32);
        } else {
            break;
        }
    }
    let p_exp = exp_val * exp_sign; // 2 的指数，直接作用于整个尾数

    // 计算尾数: int_part + frac_part / 16^frac_digits
    let scaled_int = int_part as f64;
    let scaled_frac = if frac_digits > 0 {
        frac_part as f64 / (16u64.pow(frac_digits) as f64)
    } else {
        0.0
    };

    let mantissa = scaled_int + scaled_frac;

    // 应用 2 的指数: value = mantissa * 2^p_exp
    let result = mantissa * pow2i(p_exp);
    if result.is_infinite() && mantissa.is_finite() && mantissa != 0.0 {
        Some((result, true))
    } else {
        Some((result, false))
    }
}

/// 使用重复平方法计算 2 的整数次方（no_std 替代 f64::powi）。
fn pow2i(exp: i32) -> f64 {
    let mut result = 1.0f64;
    let mut base = 2.0f64;
    let mut e = exp.unsigned_abs();
    while e > 0 {
        if e & 1 == 1 {
            result *= base;
        }
        e >>= 1;
        base *= base;
    }
    if exp >= 0 { result } else { 1.0 / result }
}

// ---------- 核心 strtod 实现 ----------

/// 内部：将 `s` 转换为浮点数。返回 (值, 是否溢出)。
unsafe fn strtox_inner(s: *const c_char, endptr: *mut *mut c_char) -> (f64, bool) {
    let bytes = unsafe { core::ffi::CStr::from_ptr(s) }.to_bytes();
    let len = bytes.len();
    let mut pos = 0;

    // 跳过空白
    skip_ws(bytes, &mut pos);

    if pos >= len {
        if !endptr.is_null() {
            *endptr = s as *mut c_char;
        }
        return (0.0, false);
    }

    let num_start = pos;

    // 扫描数字结束位置
    match scan_float_end(bytes, num_start) {
        Some(end) if end > num_start => {
            let slice = &bytes[num_start..end];

            // 特别处理：只有符号没有数字
            if slice.len() <= 2 && (slice == b"+" || slice == b"-" || slice == b"."
                || slice == b"+." || slice == b"-.") {
                if !endptr.is_null() {
                    *endptr = s as *mut c_char;
                }
                return (0.0, false);
            }

            // 优先尝试 hex float 解析
            if slice.len() >= 2 && slice[0] == b'0' && (slice[1] == b'x' || slice[1] == b'X') {
                if let Some((val, overflow)) = parse_hex_float(slice) {
                    if !endptr.is_null() {
                        *endptr = unsafe { (s as *mut u8).add(end) as *mut c_char };
                    }
                    if overflow {
                        set_erange();
                    }
                    return (val, overflow);
                }
                // hex 解析失败，fallthrough 到 from_str
            }

            // 使用 FromStr 解析（适用于十进制浮点数、inf、nan）
            let s_str = unsafe { core::str::from_utf8_unchecked(slice) };
            match f64::from_str(s_str) {
                Ok(val) => {
                    if !endptr.is_null() {
                        *endptr = unsafe { (s as *mut u8).add(end) as *mut c_char };
                    }
                    if val.is_infinite() {
                        let has_digits = slice.iter().any(|&b| {
                            b.is_ascii_digit() || b == b'.' || b == b'x' || b == b'X'
                        });
                        if has_digits {
                            set_erange();
                            return (val, true);
                        }
                    }
                    (val, false)
                }
                Err(_) => {
                    let fallback = try_shorten_and_parse(bytes, num_start, end, endptr, s);
                    if fallback.0.is_nan() || fallback.0 == 0.0 {
                        if !endptr.is_null() {
                            *endptr = s as *mut c_char;
                        }
                    }
                    fallback
                }
            }
        }
        _ => {
            if !endptr.is_null() {
                *endptr = s as *mut c_char;
            }
            (0.0, false)
        }
    }
}

/// 从较短的子串尝试解析，步进缩短直到成功或耗尽。
fn try_shorten_and_parse(
    bytes: &[u8],
    num_start: usize,
    scan_end: usize,
    endptr: *mut *mut c_char,
    orig_s: *const c_char,
) -> (f64, bool) {
    let mut end = scan_end;
    while end > num_start {
        let slice = &bytes[num_start..end];
        let s_str = unsafe { core::str::from_utf8_unchecked(slice) };
        if s_str == "+" || s_str == "-" || s_str == "." || s_str == "+." || s_str == "-." {
            end -= 1;
            continue;
        }
        match f64::from_str(s_str) {
            Ok(val) => {
                if !endptr.is_null() {
                    unsafe { *endptr = (orig_s as *mut u8).add(end) as *mut c_char; }
                }
                if val.is_infinite() && slice.iter().any(|&b| b.is_ascii_digit() || b == b'.' || b == b'x' || b == b'X') {
                    unsafe { set_erange(); }
                    return (val, true);
                }
                return (val, false);
            }
            Err(_) => {
                end -= 1;
            }
        }
    }
    (0.0, false)
}

// ---------- 公开 API ----------

/// 将 `s` 转换为 `f64`。
#[no_mangle]
pub unsafe extern "C" fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    let (val, _) = strtox_inner(s, endptr);
    val
}

/// 将 `s` 转换为 `f32`。
#[no_mangle]
pub unsafe extern "C" fn strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32 {
    let (val, _) = strtox_inner(s, endptr);
    val as f32
}

/// 将 `s` 转换为 `f64`（long double 精度，当前以 f64 表示）。
#[no_mangle]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    let (val, _) = strtox_inner(s, endptr);
    val
}

// ---------- 测试 ----------
