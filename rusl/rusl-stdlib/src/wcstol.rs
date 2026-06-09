//! wcstol 族 —— 将宽字符串转换为长整数。对外导出 C ABI 兼容的符号。
//!
//! 实现将宽字符串转换为窄字节字符串后委托给 strtol 族函数。

#![allow(unused_imports, unused_variables)]

/// C 语言 wchar_t 类型（在 Linux 上为 32 位整数）。
type wchar_t = i32;

use core::ffi::c_char;

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

// ---------- 有符号版本 ----------

/// 内部：有符号宽字符整数解析。
unsafe fn wcstox_signed(
    s: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: i32,
) -> i64 {
    // 1. 跳过前导空白
    let mut t = s;
    while *t != 0 && is_wspace(*t) {
        t = t.offset(1);
    }

    // 2. 转换为窄字节缓冲区
    let mut buf = [0u8; 2048];
    let mut i: usize = 0;
    let mut p = t;
    while *p != 0 && i < buf.len() - 1 {
        buf[i] = wc_to_byte(*p);
        i += 1;
        p = p.offset(1);
    }
    buf[i] = 0;

    // 3. 调用 strtol 解析
    let mut byte_end: *mut c_char = core::ptr::null_mut();
    let result = super::strtol::strtol(buf.as_ptr() as *const c_char, &mut byte_end, base);

    // 4. 计算宽字符串 endptr
    if !endptr.is_null() {
        if byte_end.is_null() || byte_end == buf.as_ptr() as *mut c_char {
            *endptr = s as *mut wchar_t;
        } else {
            let bytes_consumed = (byte_end as usize).wrapping_sub(buf.as_ptr() as usize);
            *endptr = t.add(bytes_consumed) as *mut wchar_t;
        }
    }

    result
}

/// 内部：有符号宽字符整数解析（strtoll 版本，与 strtol 行为相同）。
unsafe fn wcstox_signed_ll(
    s: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: i32,
) -> i64 {
    wcstox_signed(s, endptr, base)
}

// ---------- 无符号版本 ----------

/// 内部：无符号宽字符整数解析。
unsafe fn wcstox_unsigned(
    s: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: i32,
) -> u64 {
    let mut t = s;
    while *t != 0 && is_wspace(*t) {
        t = t.offset(1);
    }

    let mut buf = [0u8; 2048];
    let mut i: usize = 0;
    let mut p = t;
    while *p != 0 && i < buf.len() - 1 {
        buf[i] = wc_to_byte(*p);
        i += 1;
        p = p.offset(1);
    }
    buf[i] = 0;

    let mut byte_end: *mut c_char = core::ptr::null_mut();
    let result = super::strtol::strtoul(buf.as_ptr() as *const c_char, &mut byte_end, base);

    if !endptr.is_null() {
        if byte_end.is_null() || byte_end == buf.as_ptr() as *mut c_char {
            *endptr = s as *mut wchar_t;
        } else {
            let bytes_consumed = (byte_end as usize).wrapping_sub(buf.as_ptr() as usize);
            *endptr = t.add(bytes_consumed) as *mut wchar_t;
        }
    }

    result
}

/// 内部：无符号宽字符整数解析（strtoull 版本）。
unsafe fn wcstox_unsigned_ll(
    s: *const wchar_t,
    endptr: *mut *mut wchar_t,
    base: i32,
) -> u64 {
    wcstox_unsigned(s, endptr, base)
}

// ---------- 公开 API ----------

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `i64`。
#[no_mangle]
pub extern "C" fn wcstol(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64 {
    unsafe { wcstox_signed(s, endptr, base) }
}

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `i64`（long long 版）。
#[no_mangle]
pub extern "C" fn wcstoll(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64 {
    unsafe { wcstox_signed_ll(s, endptr, base) }
}

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `u64`（unsigned long 版）。
#[no_mangle]
pub extern "C" fn wcstoul(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64 {
    unsafe { wcstox_unsigned(s, endptr, base) }
}

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `u64`（unsigned long long 版）。
#[no_mangle]
pub extern "C" fn wcstoull(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64 {
    unsafe { wcstox_unsigned_ll(s, endptr, base) }
}

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `i64`（intmax_t 版）。
#[no_mangle]
pub extern "C" fn wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> i64 {
    unsafe { wcstox_signed(s, endptr, base) }
}

/// 将 `s` 指向的宽字符串按 `base` 进制转换为 `u64`（uintmax_t 版）。
#[no_mangle]
pub extern "C" fn wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: i32) -> u64 {
    unsafe { wcstox_unsigned(s, endptr, base) }
}

// ---------- 测试 ----------
