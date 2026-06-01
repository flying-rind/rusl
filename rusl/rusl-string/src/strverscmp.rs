//! strverscmp — 比较两个字符串的"版本号顺序"（GNU 风格），以自然方式处理数字序列（"file1" < "file10"）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 检查字符是否为数字
fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

/// strverscmp — 比较两个字符串的"版本号顺序"（GNU 风格），以自然方式处理数字序列（"file1" < "file10"）。
///
/// # Safety
/// - `l0` 非空、`r0` 非空
/// - l0 和 r0 以 null 结尾
#[no_mangle]
pub unsafe extern "C" fn strverscmp(l0: *const core::ffi::c_char, r0: *const core::ffi::c_char) -> core::ffi::c_int {
    let l = l0 as *const u8;
    let r = r0 as *const u8;
    let mut i = 0usize;
    loop {
        let lc = unsafe { *l.add(i) };
        let rc = unsafe { *r.add(i) };
        if lc != rc {
            // 检查是否都在数字段中
            let l_digit = is_digit(lc);
            let r_digit = is_digit(rc);
            if l_digit && r_digit {
                // 两个都是数字，按版本号规则比较
                // 先跳过前导零
                let mut li = i;
                let mut ri = i;
                // 找到数字段的起始（如果有前导零）
                // 跳过 l 的前导零
                while li > 0 && is_digit(unsafe { *l.add(li - 1) }) {
                    li -= 1;
                }
                while ri > 0 && is_digit(unsafe { *r.add(ri - 1) }) {
                    ri -= 1;
                }
                // 比较数字段长度
                let mut l_len = 0usize;
                while is_digit(unsafe { *l.add(li + l_len) }) {
                    l_len += 1;
                }
                let mut r_len = 0usize;
                while is_digit(unsafe { *r.add(ri + r_len) }) {
                    r_len += 1;
                }
                if l_len != r_len {
                    return (l_len as i32) - (r_len as i32);
                }
                // 长度相同，逐位比较
                for j in 0..l_len {
                    let lv = unsafe { *l.add(li + j) };
                    let rv = unsafe { *r.add(ri + j) };
                    if lv != rv {
                        return (lv as i32) - (rv as i32);
                    }
                }
                // 数字相同，但原始字符不同（如 "a1" vs "a1" 不会到这里）
                // 继续比较后面的字符
                i = li + l_len;
                continue;
            }
            // 普通字符比较（前面非数字或只有一个是数字）
            return (lc as i32) - (rc as i32);
        }
        if lc == 0 {
            return 0;
        }
        i += 1;
    }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strverscmp_impl(l: &[u8], r: &[u8]) -> core::ffi::c_int {
    let mut i = 0;
    loop {
        let lc = l.get(i).copied().unwrap_or(0);
        let rc = r.get(i).copied().unwrap_or(0);
        if lc != rc {
            let l_digit = is_digit(lc);
            let r_digit = is_digit(rc);
            if l_digit && r_digit {
                let mut li = i;
                let mut ri = i;
                while li > 0 && is_digit(l[li - 1]) {
                    li -= 1;
                }
                while ri > 0 && is_digit(r[ri - 1]) {
                    ri -= 1;
                }
                let mut l_len = 0usize;
                while li + l_len < l.len() && is_digit(l[li + l_len]) {
                    l_len += 1;
                }
                let mut r_len = 0usize;
                while ri + r_len < r.len() && is_digit(r[ri + r_len]) {
                    r_len += 1;
                }
                if l_len != r_len {
                    return (l_len as i32) - (r_len as i32);
                }
                for j in 0..l_len {
                    let lv = l[li + j];
                    let rv = r[ri + j];
                    if lv != rv {
                        return (lv as i32) - (rv as i32);
                    }
                }
                i = li + l_len;
                continue;
            }
            return (lc as i32) - (rc as i32);
        }
        if lc == 0 {
            return 0;
        }
        i += 1;
    }
}
