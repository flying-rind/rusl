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

use crate::file::FILE;
use crate::intscan::{shgetc, shunget};

// ---- floatscan 常量 (x86_64, LDBL_MANT_DIG=64) ----

const LD_B1B_DIG: usize = 3;
const LD_B1B_MAX: [u32; 3] = [18, 446744073, 709551615];
const KMAX: usize = 2048;
const MASK: usize = KMAX - 1;

const EINVAL: i32 = 22;
const ERANGE: i32 = 34;

// ---- C ABI: __floatscan ----

/// 内部实现: 返回 f64 (xmm0)，由 C 包装层 `__floatscan` 转换为 long double (st(0))。
#[no_mangle]
pub unsafe extern "C" fn __floatscan_impl(
    f: *mut FILE,
    prec: core::ffi::c_int,
    pok: core::ffi::c_int,
) -> f64 {
    unsafe { __floatscan_inner(f, prec, pok) }
}

unsafe fn __floatscan_inner(f: *mut FILE, prec: i32, pok: i32) -> f64 {
    // 字符串扫描快速路径 (rend == (void*)-1)
    let start = (*f).rpos;
    if !start.is_null() && (*f).rend as usize == !0usize {
        let mut end = start;
        while *end != 0 { end = end.add(1); }
        let len = end.offset_from(start) as usize;
        let slice = core::slice::from_raw_parts(start, len);
        let pok_flag = if pok != 0 { POK_SPECIAL } else { 0 };
        let (val, consumed) = floatscan(slice, 0, pok_flag);
        (*f).rpos = start.add(consumed);
        let buf = (*f).buf;
        (*f).shcnt = buf.offset_from((*f).rpos) as i64;
        return val;
    }

    // 真实 FILE 流 — 逐字符解析
    let (bits, emin) = match prec {
        0 => (24, -149),
        1 => (53, -1074),
        2 => (64, -16445),
        _ => return 0.0,
    };

    // 跳过空白
    let mut c: i32 = 0;
    loop {
        c = shgetc(f) as i32;
        if !is_space(c as u8) { break; }
    }

    // 正负号
    let mut sign: f64 = 1.0;
    if c == '+' as i32 || c == '-' as i32 {
        if c == '-' as i32 { sign = -1.0; }
        c = shgetc(f) as i32;
    }

    // 尝试匹配 "inf" / "infinity"
    let mut i: usize = 0;
    while i < 8 && (c | 32) == b"infinity"[i] as i32 {
        if i < 7 { c = shgetc(f) as i32; }
        i += 1;
    }
    if i == 3 || i == 8 || (i > 3 && pok != 0) {
        if i != 8 {
            shunget(f);
            if pok != 0 { for _ in 3..i { shunget(f); } }
        }
        return sign * f64::INFINITY;
    }

    // 尝试匹配 "nan"
    if i == 0 {
        i = 0;
        while i < 3 && (c | 32) == b"nan"[i] as i32 {
            if i < 2 { c = shgetc(f) as i32; }
            i += 1;
        }
    }
    if i == 3 {
        if shgetc(f) != '(' as i32 as u8 {
            shunget(f);
            return f64::NAN;
        }
        loop {
            c = shgetc(f) as i32;
            if (c as u8).wrapping_sub(b'0') < 10
                || (c as u8).wrapping_sub(b'A') < 26
                || (c as u8).wrapping_sub(b'a') < 26
                || c == '_' as i32
            {
                continue;
            }
            if c == ')' as i32 { return f64::NAN; }
            shunget(f);
            if pok == 0 {
                *crate::import::__errno_location() = EINVAL;
                return 0.0;
            }
            while i > 0 { shunget(f); i -= 1; }
            return f64::NAN;
        }
    }

    // 无效输入
    if i > 0 {
        shunget(f);
        *crate::import::__errno_location() = EINVAL;
        return 0.0;
    }

    // 十六进制前缀
    if c == '0' as i32 {
        c = shgetc(f) as i32;
        if (c | 32) == 'x' as i32 {
            return hexfloat(f, bits, emin, sign, pok);
        }
        shunget(f);
        c = '0' as i32;
    }

    decfloat(f, c, bits, emin, sign, pok)
}

// ---- hexfloat ----

unsafe fn hexfloat(f: *mut FILE, bits: i32, emin: i32, sign: f64, pok: i32) -> f64 {
    let mut x: u32 = 0;
    let mut y: f64 = 0.0;
    let mut scale: f64 = 1.0;
    let mut bias: f64 = 0.0;
    let mut gottail: bool = false;
    let mut gotrad: bool = false;
    let mut gotdig: bool = false;
    let mut rp: i64 = 0;
    let mut dc: i64 = 0;
    let mut e2: i64 = 0;

    let mut c: i32 = shgetc(f) as i32;

    // 跳过前导零
    while c == '0' as i32 { gotdig = true; c = shgetc(f) as i32; }

    if c == '.' as i32 {
        gotrad = true;
        c = shgetc(f) as i32;
        while c == '0' as i32 { gotdig = true; rp -= 1; c = shgetc(f) as i32; }
    }

    loop {
        let cv = c as u8;
        if cv.wrapping_sub(b'0') < 10 || (cv | 32).wrapping_sub(b'a') < 6 || c == '.' as i32 {
            if c == '.' as i32 {
                if gotrad { break; }
                rp = dc;
                gotrad = true;
            } else {
                gotdig = true;
                let d = if c > '9' as i32 { ((cv | 32) + 10) - b'a' } else { cv - b'0' };
                if dc < 8 {
                    x = x * 16 + d as u32;
                } else if dc < (bits / 4 + 1) as i64 {
                    y += (d as f64) * (scale / 16.0);
                    scale /= 16.0;
                } else if d > 0 && !gottail {
                    y += 0.5 * scale;
                    gottail = true;
                }
                dc += 1;
            }
        } else {
            break;
        }
        c = shgetc(f) as i32;
    }

    if !gotdig {
        shunget(f);
        if pok != 0 {
            shunget(f);
            if gotrad { shunget(f); }
        }
        return sign * 0.0;
    }

    if !gotrad { rp = dc; }
    while dc < 8 { x *= 16; dc += 1; }

    if (c | 32) == 'p' as i32 {
        e2 = scanexp(f, pok);
        if e2 == i64::MIN {
            if pok != 0 {
                shunget(f);
            }
            e2 = 0;
        }
    } else {
        shunget(f);
    }
    e2 += 4 * rp - 32;

    if x == 0 { return sign * 0.0; }
    if e2 > -(emin as i64) {
        *crate::import::__errno_location() = ERANGE;
        return sign * f64::MAX * f64::MAX;
    }
    if e2 < (emin - 2 * bits) as i64 {
        *crate::import::__errno_location() = ERANGE;
        return sign * f64::MIN_POSITIVE * f64::MIN_POSITIVE;
    }

    while x < 0x8000_0000 {
        if y >= 0.5 {
            x += x + 1;
            y += y - 1.0;
        } else {
            x += x;
            y += y;
        }
        e2 -= 1;
    }

    let mut b: i32 = bits;
    if b > (32i64 + e2 - emin as i64) as i32 {
        b = (32i64 + e2 - emin as i64) as i32;
        if b < 0 { b = 0; }
    }

    if b < bits {
        bias = copysign(scalbn(1.0, 32 + bits - b - 1), sign);
    }

    if b < 32 && y != 0.0 && (x & 1) == 0 { x += 1; y = 0.0; }

    let mut result = bias + sign * (x as f64) + sign * y;
    result -= bias;

    if result == 0.0 { *crate::import::__errno_location() = ERANGE; }

    scalbn(result, e2 as i32)
}

// ---- scanexp ----

unsafe fn scanexp(f: *mut FILE, pok: i32) -> i64 {
    let mut c: i32 = shgetc(f) as i32;
    let mut neg: bool = false;

    if c == '+' as i32 || c == '-' as i32 {
        neg = c == '-' as i32;
        c = shgetc(f) as i32;
        if (c as u8).wrapping_sub(b'0') >= 10 && pok != 0 { shunget(f); }
    }
    if (c as u8).wrapping_sub(b'0') >= 10 {
        shunget(f);
        return i64::MIN;
    }

    let mut x: i32 = 0;
    while (c as u8).wrapping_sub(b'0') < 10 && x < i32::MAX / 10 {
        x = 10 * x + (c - '0' as i32);
        c = shgetc(f) as i32;
    }
    let mut y = x as i64;
    while (c as u8).wrapping_sub(b'0') < 10 && y < i64::MAX / 100 {
        y = 10i64 * y + ((c as i64) - (b'0' as i64));
        c = shgetc(f) as i32;
    }
    while (c as u8).wrapping_sub(b'0') < 10 { c = shgetc(f) as i32; }
    shunget(f);
    if neg { -y } else { y }
}

// ---- decfloat ----

unsafe fn decfloat(f: *mut FILE, first_c: i32, bits: i32, emin: i32, sign: f64, pok: i32) -> f64 {
    let mut x: [u32; KMAX] = [0u32; KMAX];
    let th: &[u32] = &LD_B1B_MAX;
    let mut c: i32 = first_c;
    let mut lrp: i64 = 0;
    let mut dc: i64 = 0;
    let mut e10: i64 = 0;
    let mut lnz: i64 = 0;
    let mut gotdig: bool = false;
    let mut gotrad: bool = false;
    let mut denormal: bool = false;

    let mut j: usize = 0;
    let mut k: usize = 0;

    // 跳过前导零
    while c == '0' as i32 { gotdig = true; c = shgetc(f) as i32; }
    if c == '.' as i32 {
        gotrad = true;
        c = shgetc(f) as i32;
        while c == '0' as i32 { gotdig = true; lrp -= 1; c = shgetc(f) as i32; }
    }

    x[0] = 0;
    loop {
        let cv = c as u8;
        if cv.wrapping_sub(b'0') < 10 || c == '.' as i32 {
            if c == '.' as i32 {
                if gotrad { break; }
                gotrad = true;
                lrp = dc;
            } else if k < KMAX - 3 {
                dc += 1;
                if c != '0' as i32 { lnz = dc; }
                if j > 0 { x[k] = x[k] * 10 + (cv - b'0') as u32; }
                else { x[k] = (cv - b'0') as u32; }
                j += 1;
                if j == 9 { k += 1; j = 0; }
                gotdig = true;
            } else {
                dc += 1;
                if c != '0' as i32 {
                    lnz = (KMAX - 4) as i64 * 9;
                    x[KMAX - 4] |= 1;
                }
            }
        } else {
            break;
        }
        c = shgetc(f) as i32;
    }
    if !gotrad { lrp = dc; }

    if gotdig && (c | 32) == 'e' as i32 {
        e10 = scanexp(f, pok);
        if e10 == i64::MIN {
            if pok != 0 { shunget(f); }
            else { return 0.0; }
            e10 = 0;
        }
        lrp += e10;
    } else if c >= 0 {
        shunget(f);
    }
    if !gotdig {
        *crate::import::__errno_location() = EINVAL;
        return 0.0;
    }

    // 零值
    if x[0] == 0 { return sign * 0.0; }

    // 小整数优化
    if lrp == dc && dc < 10 && (bits > 30 || (x[0] as i64 >> bits) == 0) {
        return sign * (x[0] as f64);
    }
    if lrp > -(emin as i64) / 2 {
        *crate::import::__errno_location() = ERANGE;
        return sign * f64::MAX * f64::MAX;
    }
    if lrp < (emin - 2 * bits) as i64 {
        *crate::import::__errno_location() = ERANGE;
        return sign * f64::MIN_POSITIVE * f64::MIN_POSITIVE;
    }

    // 对齐不完整的 B1B 数字
    if j > 0 {
        while j < 9 { x[k] *= 10; j += 1; }
        k += 1;
    }

    let mut a: usize = 0;
    let mut z: usize = k;
    let mut e2: i64 = 0;
    let mut rp: i64 = lrp;

    // 中小整数优化
    if lnz < 9 && lnz <= rp && rp < 18 {
        if rp == 9 { return sign * (x[0] as f64); }
        if rp < 9 {
            let p10s: [f64; 8] = [10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0, 100000000.0];
            return sign * (x[0] as f64) / p10s[(8 - rp as usize)];
        }
        let bitlim = bits - 3 * (rp as i32 - 9);
        if bitlim > 30 || (x[0] as i64 >> bitlim) == 0 {
            let p10s: [f64; 8] = [10.0, 100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0, 100000000.0];
            return sign * (x[0] as f64) * p10s[(rp as usize - 10)];
        }
    }

    // 丢弃尾部零
    while z > 0 && x[z - 1] == 0 { z -= 1; }

    // 对齐小数点
    if rp % 9 != 0 {
        let rpm9 = if rp >= 0 { (rp % 9) as usize } else { ((rp % 9) + 9) as usize };
        let p10s: [u32; 9] = [1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000];
        let p10 = p10s[8 - rpm9] as u64;
        let mut carry: u32 = 0;
        let mut k2 = a;
        while k2 != z {
            let tmp = (x[k2] as u64) % p10;
            x[k2] = ((x[k2] as u64) / p10) as u32 + carry;
            carry = (1000000000 / (p10 as u32)) * (tmp as u32);
            if k2 == a && x[k2] == 0 {
                a = (a + 1) & MASK;
                rp -= 9;
            }
            k2 = (k2 + 1) & MASK;
        }
        if carry > 0 { x[z] = carry; z = (z + 1) & MASK; }
        rp += 9 - rpm9 as i64;
    }

    // 上缩放
    while rp < 9 * LD_B1B_DIG as i64 || (rp == 9 * LD_B1B_DIG as i64 && x[a] < th[0]) {
        let mut carry: u32 = 0;
        e2 -= 29;
        let mut k2 = (z.wrapping_sub(1)) & MASK;
        loop {
            let tmp: u64 = ((x[k2] as u64) << 29) + carry as u64;
            if tmp > 1000000000 {
                carry = (tmp / 1000000000) as u32;
                x[k2] = (tmp % 1000000000) as u32;
            } else {
                carry = 0;
                x[k2] = tmp as u32;
            }
            if k2 == (z.wrapping_sub(1)) & MASK && k2 != a && x[k2] == 0 { z = k2; }
            if k2 == a { break; }
            k2 = (k2.wrapping_sub(1)) & MASK;
        }
        if carry > 0 {
            rp += 9;
            a = (a.wrapping_sub(1)) & MASK;
            if a == z {
                z = (z.wrapping_sub(1)) & MASK;
                x[(z.wrapping_sub(1)) & MASK] |= x[z];
            }
            x[a] = carry;
        }
    }

    // 下缩放
    loop {
        let mut carry: u32 = 0;
        let mut sh: i32 = 1;
        let mut i: usize = 0;
        while i < LD_B1B_DIG {
            let k2 = (a + i) & MASK;
            if k2 == z || x[k2] < th[i] { i = LD_B1B_DIG; break; }
            if x[(a + i) & MASK] > th[i] { break; }
            i += 1;
        }
        if i == LD_B1B_DIG && rp == 9 * LD_B1B_DIG as i64 { break; }
        if rp > 9 + 9 * LD_B1B_DIG as i64 { sh = 9; }
        e2 += sh as i64;
        let mut k2 = a;
        while k2 != z {
            let tmp = x[k2] & ((1u32 << sh) - 1);
            x[k2] = (x[k2] >> sh) + carry;
            carry = (1000000000 >> sh) * tmp;
            if k2 == a && x[k2] == 0 {
                a = (a + 1) & MASK;
                if i > 0 { i -= 1; }
                rp -= 9;
            }
            k2 = (k2 + 1) & MASK;
        }
        if carry > 0 {
            if (z + 1) & MASK != a {
                x[z] = carry;
                z = (z + 1) & MASK;
            } else {
                x[(z.wrapping_sub(1)) & MASK] |= 1;
            }
        }
    }

    // 组装浮点数
    let mut y: f64 = 0.0;
    let mut i: usize = 0;
    while i < LD_B1B_DIG {
        if (a + i) & MASK == z {
            x[z] = 0;
            z = (z + 1) & MASK;
        }
        y = 1000000000.0 * y + x[(a + i) & MASK] as f64;
        i += 1;
    }

    y *= sign;

    let mut b = bits;
    if b > LDBL_MANT_DIG + (e2 as i32) - emin {
        b = LDBL_MANT_DIG + (e2 as i32) - emin;
        if b < 0 { b = 0; }
        denormal = true;
    }

    let mut frac: f64 = 0.0;
    let mut bias: f64 = 0.0;
    if b < LDBL_MANT_DIG {
        bias = copysign(scalbn(1.0, 2 * LDBL_MANT_DIG - b - 1), y);
        frac = fmod(y, scalbn(1.0, LDBL_MANT_DIG - b));
        y -= frac;
        y += bias;
    }

    if (a + i) & MASK != z {
        let t = x[(a + i) & MASK];
        if t < 500000000 && (t > 0 || (a + i + 1) & MASK != z) {
            frac += 0.25 * sign;
        } else if t > 500000000 {
            frac += 0.75 * sign;
        } else if t == 500000000 {
            if (a + i + 1) & MASK == z { frac += 0.5 * sign; }
            else { frac += 0.75 * sign; }
        }
        if LDBL_MANT_DIG - b >= 2 && fmod(frac, 1.0) == 0.0 { frac += 1.0; }
    }

    y += frac;
    y -= bias;

    let e2_biased = e2 + (LDBL_MANT_DIG as i64);
    if (e2_biased & (i32::MAX as i64)) > ((emax5() - 5) as i64) {
        if fabs(y) >= 2.0 / LDBL_EPSILON {
            if denormal && b == LDBL_MANT_DIG + (e2 as i32) - emin { denormal = false; }
            y *= 0.5;
            e2 += 1;
        }
        if (e2 + (LDBL_MANT_DIG as i64)) > (emax5() as i64) || (denormal && frac != 0.0) {
            *crate::import::__errno_location() = ERANGE;
        }
    }

    scalbn(y, e2 as i32)
}

// ---- math helpers (delegate to C 80-bit precision) ----

use crate::import::{
    __floatscan_scale as scalbn,
    __floatscan_mul as ldmul,
    __floatscan_abs as fabs,
    __floatscan_copysign as copysign,
    __floatscan_fmod as fmod,
};

const LDBL_MANT_DIG: i32 = 64;
const LDBL_EPSILON: f64 = 1.08420217248550443401e-19;

fn emax5() -> i32 { 16384 - 5 }

// ---- C ABI 导出结束 ----

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
