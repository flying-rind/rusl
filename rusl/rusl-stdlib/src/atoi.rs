//! atoi/atol/atoll —— 将字符串转换为整数。对外导出 C ABI 兼容的符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

// ---------- 内部辅助函数 ----------

/// 检查字符是否为空白（isspace 的简版）。
#[inline]
fn isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// 检查字符是否为数字（isdigit 的简版）。
#[inline]
fn isdigit(c: u8) -> bool {
    matches!(c, b'0'..=b'9')
}

/// 跳过前导空白，读符号，返回（修正后的指针, 是否为负）。
#[inline]
unsafe fn skip_whitespace_and_sign(s: &mut *const c_char) -> bool {
    while isspace(**s as u8) {
        *s = (*s).add(1);
    }
    match **s as u8 {
        b'-' => {
            *s = (*s).add(1);
            true
        }
        b'+' => {
            *s = (*s).add(1);
            false
        }
        _ => false,
    }
}

/// 负向累加（i32 版）：持续累加直到遇到非数字字符。
/// 中间值始终 ≤ 0，保证安全解析 i32::MIN。
#[inline]
unsafe fn neg_accum_i32(s: &mut *const c_char) -> i32 {
    let mut n: i32 = 0;
    while isdigit(**s as u8) {
        n = n.wrapping_mul(10).wrapping_sub((**s as u8 - b'0') as i32);
        *s = (*s).add(1);
    }
    n
}

/// 负向累加（i64 版）。
#[inline]
unsafe fn neg_accum_i64(s: &mut *const c_char) -> i64 {
    let mut n: i64 = 0;
    while isdigit(**s as u8) {
        n = n.wrapping_mul(10).wrapping_sub((**s as u8 - b'0') as i64);
        *s = (*s).add(1);
    }
    n
}

// ---------- 公开 API ----------

/// 将 `s` 指向的以 null 结尾的字符串转换为 `i32`。
///
/// # Safety
///
/// - `s` 必须指向以 null 结尾的有效 C 字符串。
///
/// # 返回值
///
/// - 解析成功：返回对应的 `i32` 值。
/// - 无有效数字：返回 `0`。
/// - 溢出：行为未定义。
///
/// 内部采用负向累加策略，中间值始终 <= 0，以安全解析 TYPE_MIN。
#[no_mangle]
pub unsafe extern "C" fn atoi(s: *const c_char) -> i32 {
    let mut p = s;
    let neg = skip_whitespace_and_sign(&mut p);
    let n = neg_accum_i32(&mut p);
    if neg { n } else { -n }
}

/// 将 `s` 指向的以 null 结尾的字符串转换为 `i64`。
///
/// # Safety
///
/// - `s` 必须指向以 null 结尾的有效 C 字符串。
///
/// # 返回值
///
/// - 解析成功：返回对应的 `i64` 值。
/// - 无有效数字：返回 `0`。
/// - 溢出：行为未定义。
///
/// 内部采用负向累加策略，中间值始终 <= 0，以安全解析 TYPE_MIN。
#[no_mangle]
pub unsafe extern "C" fn atol(s: *const c_char) -> i64 {
    let mut p = s;
    let neg = skip_whitespace_and_sign(&mut p);
    let n = neg_accum_i64(&mut p);
    if neg { n } else { -n }
}

/// 将 `s` 指向的以 null 结尾的字符串转换为 `i64`。
///
/// # Safety
///
/// - `s` 必须指向以 null 结尾的有效 C 字符串。
///
/// # 返回值
///
/// - 解析成功：返回对应的 `i64` 值。
/// - 无有效数字：返回 `0`。
/// - 溢出：行为未定义。
///
/// 内部采用负向累加策略，中间值始终 <= 0，以安全解析 TYPE_MIN。
#[no_mangle]
pub unsafe extern "C" fn atoll(s: *const c_char) -> i64 {
    atol(s)
}
