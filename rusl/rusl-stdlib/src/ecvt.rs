//! ecvt/fcvt/gcvt —— 浮点数格式转换（已过时的 GNU 扩展）。对外导出 C ABI 兼容的符号。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::fmt::Write;

// ---------- 内部辅助：FmtBuf ----------

/// 持有一个 &mut [u8] 的可写缓冲区（no_std 下替代 sprintf）。
struct FmtBuf<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Write for FmtBuf<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        if self.pos + bytes.len() > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }
}

impl<'a> FmtBuf<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        FmtBuf { buf, pos: 0 }
    }

    fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.pos]) }
    }
}

// ---------- ecvt ----------

/// ecvt 使用的静态缓冲区（非线程安全，与原 C 实现一致）。
static mut ECVT_BUF: [c_char; 16] = [0; 16];

/// fcvt 在多前导零时返回的常量 "000..." 字符串。
static ZEROS: [c_char; 16] = [
    '0' as c_char, '0' as c_char, '0' as c_char, '0' as c_char,
    '0' as c_char, '0' as c_char, '0' as c_char, '0' as c_char,
    '0' as c_char, '0' as c_char, '0' as c_char, '0' as c_char,
    '0' as c_char, '0' as c_char, '0' as c_char, 0,
];

/// 将 `x` 转换为科学计数法数字串（已过时，不推荐在新代码中使用）。
///
/// # Safety
///
/// - `dp` 必须为指向有效 `i32` 的可写指针，用于输出小数点位置。
/// - `sign` 必须为指向有效 `i32` 的可写指针，用于输出符号（0=正，1=负）。
/// - `n` 为有效数字位数（通常 <= 15）。
///
/// # 返回值
///
/// 返回指向静态缓冲区中数字串的指针（非线程安全）。
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn ecvt(x: f64, n: i32, dp: *mut i32, sign: *mut i32) -> *mut c_char {
    // 限制有效数字位数
    let n = if (n - 1) as u32 > 15 { 15 } else { n };
    let prec = (n - 1) as usize;

    // 用 Rust 格式化输出到临时缓冲区
    let mut tmp = [0u8; 64];
    let mut w = FmtBuf::new(&mut tmp);
    // 格式: {:.prec$e} = 科学计数法, prec 为小数点后位数
    let _ = write!(w, "{:.prec$e}", x, prec = prec);
    let s = w.as_str();
    let bytes = s.as_bytes();

    // 跳过符号，设置 *sign
    let mut i: usize = 0;
    *sign = if bytes[0] == b'-' { i = 1; 1 } else { 0 };

    // 复制数字（跳过小数点）到静态缓冲区
    let buf: &mut [c_char; 16] = &mut ECVT_BUF;
    let mut j: usize = 0;
    while i < bytes.len() && bytes[i] != b'e' {
        if bytes[i] != b'.' {
            buf[j] = bytes[i] as c_char;
            j += 1;
        }
        i += 1;
    }
    // null 终止
    buf[j] = 0;

    // 跳过 'e' 字符
    if i < bytes.len() && bytes[i] == b'e' {
        i += 1;
    }

    // 解析指数符号
    let mut exp_neg = false;
    if i < bytes.len() && bytes[i] == b'-' {
        exp_neg = true;
        i += 1;
    } else if i < bytes.len() && bytes[i] == b'+' {
        i += 1;
    }

    // 解析指数值
    let mut exp: i32 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        exp = exp.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i32);
        i += 1;
    }
    if exp_neg {
        exp = -exp;
    }
    *dp = exp + 1;

    buf.as_mut_ptr()
}

/// 将 `x` 转换为定点格式数字串（已过时，不推荐在新代码中使用）。
///
/// # Safety
///
/// - `dp` 必须为指向有效 `i32` 的可写指针，用于输出小数点位置。
/// - `sign` 必须为指向有效 `i32` 的可写指针，用于输出符号（0=正，1=负）。
/// - `n` 为小数位数（<= 1400）。
///
/// # 返回值
///
/// 返回指向定点格式数字串的指针。当前导零过多时返回 "000..." 常量字符串。
#[no_mangle]
pub unsafe extern "C" fn fcvt(x: f64, n: i32, dp: *mut i32, sign: *mut i32) -> *mut c_char {
    // 限制小数位数
    let n = if n > 1400 { 1400 } else { n };
    let prec = n as usize;

    // 用 Rust 格式化输出 fixed-point
    let mut tmp = [0u8; 1500];
    let mut w = FmtBuf::new(&mut tmp);
    let _ = write!(w, "{:.prec$}", x, prec = prec);
    let s = w.as_str();
    let bytes = s.as_bytes();

    // 跳过符号
    let mut i: usize = 0;
    if bytes[0] == b'-' {
        i = 1;
    }

    // 计算前导零数量 lz
    let lz: i32;
    if bytes[i] == b'0' {
        // 寻找小数点后连续的零
        let dot_pos = bytes[i..].iter().position(|&c| c == b'.').unwrap_or(0);
        let after_dot = &bytes[i + dot_pos + 1..];
        let zero_count = after_dot.iter().take_while(|&&c| c == b'0').count();
        lz = zero_count as i32;
    } else {
        // 计算整数部分位数（到小数点的距离）
        let dot_pos = bytes[i..].iter().position(|&c| c == b'.').unwrap_or(bytes.len());
        lz = -(dot_pos as i32);
    }

    // 如果前导零 >= n，返回 "000..." 字符串
    if n <= lz {
        *sign = i as i32;
        *dp = 1;
        let n = if n > 14 { 14 } else { n };
        return ZEROS.as_ptr().add((14 - n) as usize) as *mut c_char;
    }

    // 否则委托 ecvt
    ecvt(x, n - lz, dp, sign)
}

/// 将 `x` 转换为 `%g` 格式写入 `b`（已过时，不推荐在新代码中使用）。
///
/// # Safety
///
/// - `b` 必须为由调用者提供、足够容纳格式化结果的缓冲区。
/// - `n` 为有效数字位数。
///
/// # 返回值
///
/// 返回 `b` 指针。
#[no_mangle]
pub unsafe extern "C" fn gcvt(x: f64, n: i32, b: *mut c_char) -> *mut c_char {
    // 计算缓冲区长度（假设不超过 64 字节）
    let buf_len = 64usize;
    let slice = unsafe { core::slice::from_raw_parts_mut(b as *mut u8, buf_len) };
    let mut w = FmtBuf::new(slice);
    let prec = n.max(1) as usize;
    let _ = write!(w, "{:.prec$}", x, prec = prec);
    // null 终止
    unsafe {
        *b.add(w.pos) = 0;
    }
    b
}
