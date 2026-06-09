//! strverscmp — 比较两个字符串的"版本号顺序"（GNU 风格），以自然方式处理数字序列（"file1" < "file10"）。
//!
//! 算法来自 musl libc 的 strverscmp 实现，严格遵循 GNU 版本号比较语义：
//! 1. 找到最长匹配前缀，同时追踪最大数字后缀的起始位置(dp)及该后缀是否全为零(z)
//! 2. 若失配字符均为非零数字，则较长的数字序列更大
//! 3. 若共同数字前缀全为零，则数字小于非数字
//! 4. 否则按普通字符比较

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

/// strverscmp — 比较两个字符串的"版本号顺序"（GNU 风格）。
///
/// # Safety
/// - `l0` 非空、`r0` 非空
/// - l0 和 r0 以 null 结尾
#[no_mangle]
pub extern "C" fn strverscmp(l0: *const core::ffi::c_char, r0: *const core::ffi::c_char) -> core::ffi::c_int {
    let l = l0 as *const u8;
    let r = r0 as *const u8;
    let mut dp: usize = 0; // 当前数字后缀的起始位置
    let mut z: i32 = 1;    // 当前数字后缀是否全为零
    let mut i: usize = 0;

    // 找到最长匹配前缀，追踪数字后缀信息
    loop {
        let lc = unsafe { *l.add(i) };
        let rc = unsafe { *r.add(i) };
        if lc != rc {
            break;
        }
        if lc == 0 {
            return 0;
        }
        if !is_digit(lc) {
            dp = i + 1;
            z = 1;
        } else if lc != b'0' {
            z = 0;
        }
        i += 1;
    }

    let li = unsafe { *l.add(i) };
    let ri = unsafe { *r.add(i) };

    // 若失配字符均为非零数字，则较长的数字序列更大
    if unsafe { *l.add(dp) }.wrapping_sub(b'1') < 9
        && unsafe { *r.add(dp) }.wrapping_sub(b'1') < 9
    {
        let mut j = i;
        while is_digit(unsafe { *l.add(j) }) {
            if !is_digit(unsafe { *r.add(j) }) {
                return 1;
            }
            j += 1;
        }
        if is_digit(unsafe { *r.add(j) }) {
            return -1;
        }
    } else if z != 0 && dp < i && (is_digit(li) || is_digit(ri)) {
        // 共同数字前缀全为零时，数字小于非数字
        // 对应 musl: return (unsigned char)(l[i]-'0') - (unsigned char)(r[i]-'0')
        // 使用 wrapping_sub 模拟 C 的 unsigned char 算术（减法前减 '0'）
        let la = li.wrapping_sub(b'0') as i32;
        let ra = ri.wrapping_sub(b'0') as i32;
        return la - ra;
    }

    (li as i32) - (ri as i32)
}

/// 安全的 Rust 内部实现。
pub(crate) fn strverscmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int {
    let mut dp: usize = 0;
    let mut z: i32 = 1;
    let mut i: usize = 0;

    loop {
        let lc = *l.get(i).unwrap_or(&0);
        let rc = *r.get(i).unwrap_or(&0);
        if lc != rc {
            break;
        }
        if lc == 0 {
            return 0;
        }
        if !is_digit(lc) {
            dp = i + 1;
            z = 1;
        } else if lc != b'0' {
            z = 0;
        }
        i += 1;
    }

    let li = *l.get(i).unwrap_or(&0);
    let ri = *r.get(i).unwrap_or(&0);

    let ldp = *l.get(dp).unwrap_or(&0);
    let rdp = *r.get(dp).unwrap_or(&0);

    if ldp.wrapping_sub(b'1') < 9 && rdp.wrapping_sub(b'1') < 9 {
        let mut j = i;
        while j < l.len() && is_digit(l[j]) {
            if j >= r.len() || !is_digit(r[j]) {
                return 1;
            }
            j += 1;
        }
        if j < r.len() && is_digit(r[j]) {
            return -1;
        }
    } else if z != 0 && dp < i && (is_digit(li) || is_digit(ri)) {
        return (li as i32) - (ri as i32);
    }

    (li as i32) - (ri as i32)
}
