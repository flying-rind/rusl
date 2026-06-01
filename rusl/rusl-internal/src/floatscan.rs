//! floatscan 模块 — 浮点数扫描（字符串到浮点数转换）。
//!
//! 本模块定义了 `floatscan` 函数，是 rusl 中所有浮点扫描操作的
//! 统一后端：`scanf("%f")`、`strtod()`、`strtof()` 等都委托给它。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

// ---------------------------------------------------------------------------
// 辅助函数: isspace / pow10 / pow2 (no_std 兼容)
// ---------------------------------------------------------------------------

#[inline]
fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == b'\x0c'
}

/// no_std 兼容的 10^n 计算（二分幂法）。
fn pow10(n: i32) -> f64 {
    if n == 0 { return 1.0; }
    if n < 0 { return 1.0 / pow10(-n); }
    let half = pow10(n / 2);
    let sq = half * half;
    if n & 1 == 0 { sq } else { 10.0 * sq }
}

/// no_std 兼容的 2^n 计算（二分幂法）。
fn pow2(n: i32) -> f64 {
    if n == 0 { return 1.0; }
    if n < 0 { return 1.0 / pow2(-n); }
    let half = pow2(n / 2);
    let sq = half * half;
    if n & 1 == 0 { sq } else { 2.0 * sq }
}

// ---------------------------------------------------------------------------
// pok 标志位
// ---------------------------------------------------------------------------

const POK_HEX: u32 = 1 << 0;
const POK_SPECIAL: u32 = 1 << 1;

// ---------------------------------------------------------------------------
// 字节序列匹配 (no_std: 不使用 Vec/alloc)
// ---------------------------------------------------------------------------

fn match_prefix_icase(src: &[u8], start: usize, expected: &[u8]) -> Option<usize> {
    let n = expected.len();
    if start + n > src.len() { return None; }
    for i in 0..n {
        if src[start + i].to_ascii_lowercase() != expected[i].to_ascii_lowercase() {
            return None;
        }
    }
    Some(start + n)
}

// ---------------------------------------------------------------------------
// 特殊值匹配
// ---------------------------------------------------------------------------

fn try_match_inf(input: &[u8], start: usize) -> Option<usize> {
    if let Some(p) = match_prefix_icase(input, start, b"infinity") { return Some(p); }
    match_prefix_icase(input, start, b"inf")
}

fn try_match_nan(input: &[u8], start: usize) -> Option<usize> {
    let end = match_prefix_icase(input, start, b"nan")?;
    if end < input.len() && input[end] == b'(' {
        let mut i = end + 1;
        while i < input.len() {
            if input[i] == b')' { return Some(i + 1); }
            let c = input[i];
            if !(c.is_ascii_alphanumeric() || c == b'_') { break; }
            i += 1;
        }
        return Some(end);
    }
    Some(end)
}

// ---------------------------------------------------------------------------
// 字符分类
// ---------------------------------------------------------------------------

fn hex_digit_value(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

#[inline]
fn is_dec_digit(c: u8) -> bool { (c as char).is_ascii_digit() }

#[inline]
fn is_hex_digit(c: u8) -> bool { (c as char).is_ascii_hexdigit() }

// ---------------------------------------------------------------------------
// floatscan — 主入口
// ---------------------------------------------------------------------------

/// 从字节切片中解析 f64 浮点数。rusl 所有浮点扫描操作的统一后端。
///
/// # 返回值
///
/// `(value, consumed)` — 解析值和消费字节数，失败时 consumed = 0
pub fn floatscan(
    input: &[u8],
    prec: usize,
    pok: u32,
) -> (f64, usize) {
    if input.is_empty() { return (0.0, 0); }
    let _ = prec;
    let mut pos: usize = 0;
    let len = input.len();

    // 跳过空白
    while pos < len && is_space(input[pos]) { pos += 1; }
    if pos >= len { return (0.0, 0); }

    // 正负号
    let mut negative = false;
    if input[pos] == b'+' { pos += 1; }
    else if input[pos] == b'-' { negative = true; pos += 1; }
    if pos >= len { return (0.0, 0); }
    let sign: f64 = if negative { -1.0 } else { 1.0 };

    // 特殊值
    if (pok & POK_SPECIAL) != 0 {
        if let Some(e) = try_match_inf(input, pos) { return (sign * f64::INFINITY, e); }
        if let Some(e) = try_match_nan(input, pos) { return (f64::NAN, e); }
    }
    if pos + 3 <= len {
        let b0 = input[pos].to_ascii_lowercase();
        let b1 = input[pos + 1].to_ascii_lowercase();
        let b2 = input[pos + 2].to_ascii_lowercase();
        if (b0 == b'i' && b1 == b'n' && b2 == b'f')
            || (b0 == b'n' && b1 == b'a' && b2 == b'n')
        { return (0.0, 0); }
    }

    // 十六进制浮点数
    if (pok & POK_HEX) != 0 && pos + 1 < len && input[pos] == b'0' {
        let c = input[pos + 1];
        if c == b'x' || c == b'X' { return parse_hex_float(input, pos, sign); }
    }

    // 十进制浮点数
    parse_decimal_float(input, pos, sign)
}

// ---------------------------------------------------------------------------
// 十进制浮点数解析
// ---------------------------------------------------------------------------

fn parse_decimal_float(input: &[u8], start: usize, sign: f64) -> (f64, usize) {
    let mut pos = start;
    let len = input.len();
    if pos >= len { return (0.0, 0); }

    let mut int_part: u64 = 0;
    let mut got_digits = false;
    while pos < len && is_dec_digit(input[pos]) {
        int_part = int_part.saturating_mul(10).saturating_add((input[pos] - b'0') as u64);
        got_digits = true;
        pos += 1;
    }

    let mut frac_part: u64 = 0;
    let mut frac_digits: i32 = 0;
    if pos < len && input[pos] == b'.' {
        pos += 1;
        while pos < len && is_dec_digit(input[pos]) {
            frac_part = frac_part.saturating_mul(10).saturating_add((input[pos] - b'0') as u64);
            frac_digits += 1;
            got_digits = true;
            pos += 1;
        }
    }

    if !got_digits { return (0.0, start); }

    let mut exponent: i32 = 0;
    if pos < len && (input[pos] == b'e' || input[pos] == b'E') {
        pos += 1;
        let exp_neg = if pos < len && input[pos] == b'-' {
            pos += 1; true
        } else if pos < len && input[pos] == b'+' {
            pos += 1; false
        } else { false };
        let exp_start = pos;
        while pos < len && is_dec_digit(input[pos]) {
            exponent = exponent.saturating_mul(10).saturating_add((input[pos] - b'0') as i32);
            pos += 1;
        }
        if pos == exp_start { pos -= 1; }
        else if exp_neg { exponent = -exponent; }
    }

    let mut value = int_part as f64;
    if frac_digits > 0 {
        value += (frac_part as f64) * pow10(-frac_digits);
    }
    let total_exp = exponent; // 整数部分和小数部分已正确组合，指数直接作用于整体
    if total_exp != 0 { value *= pow10(total_exp); }
    value *= sign;
    (value, pos)
}

// ---------------------------------------------------------------------------
// 十六进制浮点数解析
// ---------------------------------------------------------------------------

fn parse_hex_float(input: &[u8], start: usize, sign: f64) -> (f64, usize) {
    let mut pos = start + 2;
    let len = input.len();

    let mut int_part: u64 = 0;
    let mut got_digits = false;
    while pos < len && is_hex_digit(input[pos]) {
        int_part = int_part.wrapping_mul(16).wrapping_add(hex_digit_value(input[pos]) as u64);
        got_digits = true; pos += 1;
    }

    let mut frac_bits: u64 = 0;
    let mut frac_count: i32 = 0;
    if pos < len && input[pos] == b'.' {
        pos += 1;
        while pos < len && is_hex_digit(input[pos]) {
            frac_bits = frac_bits.wrapping_mul(16).wrapping_add(hex_digit_value(input[pos]) as u64);
            frac_count += 1; got_digits = true; pos += 1;
        }
    }

    if !got_digits { return (0.0, start); }

    let mut exp: i32 = 0;
    if pos < len && (input[pos] == b'p' || input[pos] == b'P') {
        pos += 1;
        let exp_neg = if pos < len && input[pos] == b'-' {
            pos += 1; true
        } else if pos < len && input[pos] == b'+' {
            pos += 1; false
        } else { false };
        let exp_start = pos;
        while pos < len && is_dec_digit(input[pos]) {
            exp = exp.saturating_mul(10).saturating_add((input[pos] - b'0') as i32);
            pos += 1;
        }
        if pos == exp_start { pos -= 1; }
        else if exp_neg { exp = -exp; }
    }

    let mut value = int_part as f64;
    if frac_count > 0 {
        value += (frac_bits as f64) * pow2(-4 * frac_count);
    }
    if exp != 0 { value *= pow2(exp); }
    value *= sign;
    (value, pos)
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;

    fn check(input: &[u8], pok: u32, expected_val: f64, expected_consumed: usize) {
        let (val, consumed) = floatscan(input, 0, pok);
        if expected_val.is_nan() {
            assert!(val.is_nan(), "Expected NaN for {:?}, got {}", input, val);
        } else if expected_val.is_infinite() {
            assert_eq!(val, expected_val,
                "Value mismatch for {:?}: got {}, expected {}",
                input, val, expected_val);
        } else {
            let eps = 1e-9;
            assert!(
                (val - expected_val).abs() < eps || (val == 0.0 && expected_val == 0.0),
                "Value mismatch for {:?}: got {}, expected {}",
                input, val, expected_val
            );
        }
        assert_eq!(consumed, expected_consumed,
            "Consumed mismatch for {:?}: got {}, expected {}",
            input, consumed, expected_consumed);
    }

    test!("floatscan_zero" { check(b"0", 0, 0.0, 1); });
    test!("floatscan_simple_float" { check(b"1.5", 0, 1.5, 3); });
    test!("floatscan_negative" { check(b"-3.14", 0, -3.14, 5); });
    test!("floatscan_exponent" {
        check(b"1e10", 0, 1e10, 4);
        check(b"1e-2", 0, 0.01, 4);
    });
    test!("floatscan_integer" { check(b"42", 0, 42.0, 2); });
    test!("floatscan_infinity" {
        check(b"inf", POK_SPECIAL, f64::INFINITY, 3);
        check(b"infinity", POK_SPECIAL, f64::INFINITY, 8);
        check(b"INF", POK_SPECIAL, f64::INFINITY, 3);
    });
    test!("floatscan_nan" {
        let (v1, c1) = floatscan(b"nan", 0, POK_SPECIAL);
        assert!(v1.is_nan()); assert_eq!(c1, 3);
        let (v2, c2) = floatscan(b"nan(mytest)", 0, POK_SPECIAL);
        assert!(v2.is_nan()); assert_eq!(c2, 11);
        let (v3, c3) = floatscan(b"NAN", 0, POK_SPECIAL);
        assert!(v3.is_nan()); assert_eq!(c3, 3);
    });
    test!("floatscan_invalid" { check(b"abc", 0, 0.0, 0); });
    test!("floatscan_empty" { check(b"", 0, 0.0, 0); });
    test!("floatscan_leading_ws" { check(b"  3.14", 0, 3.14, 6); });
    test!("floatscan_positive_sign" { check(b"+2.5", 0, 2.5, 4); });
    test!("floatscan_neg_inf" { check(b"-inf", POK_SPECIAL, f64::NEG_INFINITY, 4); });
    test!("floatscan_dec_with_exp" {
        check(b"3.14e2", 0, 314.0, 6);
        let (v, c) = floatscan(b"2.5e-3", 0, 0);
        assert!((v - 0.0025).abs() < 1e-10, "got {}", v);
        assert_eq!(c, 6);
    });
    test!("floatscan_dot_fails" {
        let (v, c) = floatscan(b".", 0, 0);
        assert_eq!(v, 0.0); assert_eq!(c, 0);
    });
    test!("floatscan_dot_with_digits" { check(b".5", 0, 0.5, 2); });
    test!("floatscan_hex_float" { check(b"0x1.0p3", POK_HEX, 8.0, 7); });
    test!("floatscan_no_pok_no_special" {
        let (v, c) = floatscan(b"inf", 0, 0);
        assert_eq!(v, 0.0); assert_eq!(c, 0);
    });
    test!("floatscan_trailing" { check(b"3.14abc", 0, 3.14, 4); });
    test!("floatscan_large" {
        // 1e310 超出 f64 范围，应溢出为正无穷
        let (v, _) = floatscan(b"1e310", 0, 0);
        assert!(v.is_infinite(), "Expected infinity for 1e310, got {}", v);
    });

    test!("pow10_basic" {
        assert!((pow10(0) - 1.0).abs() < 1e-15);
        assert!((pow10(1) - 10.0).abs() < 1e-15);
        assert!((pow10(3) - 1000.0).abs() < 1e-12);
        assert!((pow10(-1) - 0.1).abs() < 1e-16);
        assert!((pow10(-2) - 0.01).abs() < 1e-17);
    });

    test!("pow2_basic" {
        assert!((pow2(0) - 1.0).abs() < 1e-15);
        assert!((pow2(3) - 8.0).abs() < 1e-15);
        assert!((pow2(-1) - 0.5).abs() < 1e-16);
    });
}
