//! strtol 族 —— 将字符串转换为长整数。对外导出 C ABI 兼容的符号。
//!
//! 纯 Rust 实现，采用负向累加策略解析有符号整数以避免 TYPE_MIN 溢出，
//! 采用正向累加策略解析无符号整数。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use rusl_errno::__errno_location;
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

// ---------- 辅助函数 ----------

/// 将字节转换为对应进制的数字值 (0-35)，无效返回 None。
#[inline]
fn digit_val(c: u8) -> Option<i64> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i64),
        b'a'..=b'z' => Some((c - b'a' + 10) as i64),
        b'A'..=b'Z' => Some((c - b'A' + 10) as i64),
        _ => None,
    }
}

/// 跳过前导空白。
#[inline]
fn skip_whitespace(bytes: &[u8], pos: &mut usize) {
    while *pos < bytes.len() && bytes[*pos].is_ascii_whitespace() {
        *pos += 1;
    }
}

// ---------- 有符号整数解析 (负向累加) ----------

/// 内部：解析有符号整数 i64。
///
/// 使用负向累加避免解析 i64::MIN 时溢出。mid value 始终 <= 0。
unsafe fn strtox_signed(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64 {
    let bytes = unsafe { core::ffi::CStr::from_ptr(s) }.to_bytes();
    let len = bytes.len();
    let mut pos = 0;

    // 1. 跳过前导空白
    skip_whitespace(bytes, &mut pos);

    if pos >= len {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    // 2. 处理符号
    let mut neg = false;
    if bytes[pos] == b'-' {
        neg = true;
        pos += 1;
    } else if bytes[pos] == b'+' {
        pos += 1;
    }

    // 3. 确定实际进制
    let actual_base: i32 = if base == 0 {
        if pos < len && bytes[pos] == b'0' {
            if pos + 1 < len && (bytes[pos + 1] == b'x' || bytes[pos + 1] == b'X') {
                pos += 2; // 跳过 "0x"
                16
            } else {
                // 不跳过 '0'，留给 digit 循环处理
                8
            }
        } else {
            10
        }
    } else if base == 16 {
        // 跳过可选的 "0x"/"0X" 前缀
        if pos + 1 < len && bytes[pos] == b'0' && (bytes[pos + 1] == b'x' || bytes[pos + 1] == b'X') {
            pos += 2;
        }
        16
    } else {
        base
    };

    // 验证进制有效性
    if actual_base < 2 || actual_base > 36 {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    let base_i64 = actual_base as i64;

    // 负向累加的 overflow 阈值
    let cutoff = i64::MIN / base_i64;
    let cutlim = -(i64::MIN % base_i64); // 正数，用于比较

    let mut val: i64 = 0;
    let mut any = false;
    let mut overflow = false;

    // 4. 解析数字 (负向累加)
    while pos < len {
        let c = bytes[pos];
        match digit_val(c) {
            Some(d) if d < base_i64 => {
                any = true;
                if overflow {
                    pos += 1;
                    continue;
                }
                if val < cutoff || (val == cutoff && d > cutlim) {
                    overflow = true;
                    pos += 1;
                    continue;
                }
                val = val * base_i64 - d;
                pos += 1;
            }
            _ => break,
        }
    }

    // 5. 处理无有效数字
    if !any {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    // 6. 设置 endptr
    if !endptr.is_null() {
        *endptr = unsafe { (s as *mut u8).add(pos) as *mut c_char };
    }

    // 7. 处理溢出
    if overflow {
        set_erange();
        return if neg { i64::MIN } else { i64::MAX };
    }

    // 8. 处理符号反转
    if !neg {
        if val == i64::MIN {
            // 负向累加达到 i64::MIN 无法安全取反 -> 正向溢出
            set_erange();
            return i64::MAX;
        }
        val = -val;
    }

    val
}

// ---------- 无符号整数解析 (正向累加) ----------

/// 内部：解析无符号整数 u64。
unsafe fn strtox_unsigned(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64 {
    let bytes = unsafe { core::ffi::CStr::from_ptr(s) }.to_bytes();
    let len = bytes.len();
    let mut pos = 0;

    // 1. 跳过前导空白
    skip_whitespace(bytes, &mut pos);

    if pos >= len {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    // 2. 处理符号
    let mut neg = false;
    if bytes[pos] == b'-' {
        neg = true;
        pos += 1;
    } else if bytes[pos] == b'+' {
        pos += 1;
    }

    // 3. 确定实际进制
    let actual_base: i32 = if base == 0 {
        if pos < len && bytes[pos] == b'0' {
            if pos + 1 < len && (bytes[pos + 1] == b'x' || bytes[pos + 1] == b'X') {
                pos += 2;
                16
            } else {
                8
            }
        } else {
            10
        }
    } else if base == 16 {
        if pos + 1 < len && bytes[pos] == b'0' && (bytes[pos + 1] == b'x' || bytes[pos + 1] == b'X') {
            pos += 2;
        }
        16
    } else {
        base
    };

    if actual_base < 2 || actual_base > 36 {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    let base_u64 = actual_base as u64;

    // 正向累加的 overflow 阈值
    let cutoff = u64::MAX / base_u64;
    let cutlim = u64::MAX % base_u64;

    let mut val: u64 = 0;
    let mut any = false;
    let mut overflow = false;

    // 4. 解析数字 (正向累加)
    while pos < len {
        let c = bytes[pos];
        match digit_val(c) {
            Some(d) if d < actual_base as i64 => {
                let d_u64 = d as u64;
                any = true;
                if overflow {
                    pos += 1;
                    continue;
                }
                if val > cutoff || (val == cutoff && d_u64 > cutlim) {
                    overflow = true;
                    pos += 1;
                    continue;
                }
                val = val * base_u64 + d_u64;
                pos += 1;
            }
            _ => break,
        }
    }

    // 5. 处理无有效数字
    if !any {
        if !endptr.is_null() { *endptr = s as *mut c_char; }
        return 0;
    }

    // 6. 设置 endptr
    if !endptr.is_null() {
        *endptr = unsafe { (s as *mut u8).add(pos) as *mut c_char };
    }

    // 7. 处理溢出
    if overflow {
        set_erange();
        return u64::MAX;
    }

    // 8. 处理符号反转
    if neg && val != 0 {
        // 二补码取反
        val = val.wrapping_neg();
        // 注意：对于 unsigned，负值不会溢出（-x = UMAX - x + 1，始终有定义）
    }

    val
}

// ---------- 公开 API ----------

/// 将 `s` 按 `base` 进制转换为 `i64`。
#[no_mangle]
pub unsafe extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64 {
    strtox_signed(s, endptr, base)
}

/// 将 `s` 按 `base` 进制转换为 `i64`（long long 版）。
#[no_mangle]
pub unsafe extern "C" fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64 {
    strtox_signed(s, endptr, base)
}

/// 将 `s` 按 `base` 进制转换为 `u64`（unsigned long 版）。
#[no_mangle]
pub unsafe extern "C" fn strtoul(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64 {
    strtox_unsigned(s, endptr, base)
}

/// 将 `s` 按 `base` 进制转换为 `u64`（unsigned long long 版）。
#[no_mangle]
pub unsafe extern "C" fn strtoull(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64 {
    strtox_unsigned(s, endptr, base)
}

/// 将 `s` 按 `base` 进制转换为 `i64`（intmax_t 版）。
#[no_mangle]
pub unsafe extern "C" fn strtoimax(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64 {
    strtox_signed(s, endptr, base)
}

/// 将 `s` 按 `base` 进制转换为 `u64`（uintmax_t 版）。
#[no_mangle]
pub unsafe extern "C" fn strtoumax(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64 {
    strtox_unsigned(s, endptr, base)
}

// ---------- 测试 ----------
