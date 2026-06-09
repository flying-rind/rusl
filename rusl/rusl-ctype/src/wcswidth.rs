//! wcswidth —— 计算宽字符串的显示列宽总和。
//! 对应 musl src/ctype/wcswidth.c
//!
//! 遍历宽字符串中的每个字符并调用 wcwidth() 累积列宽值。
//! 若遇到任何不可打印字符，立即返回 -1。

use core::ffi::c_int;

use rusl_core::c_types::{size_t, wchar_t};

// 引用同模块的 wcwidth 函数
use super::wcwidth::wcwidth;

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 计算宽字符串的显示列宽总和。
///
/// 从 `wcs` 指向的宽字符串中读取最多 `n` 个字符（或直到遇至 null 终止符），
/// 对每个字符调用 `wcwidth()` 并将返回的列宽累加。
///
/// **前置条件**:
/// - `wcs`: 指向以 null 结尾的宽字符串（不得为 NULL）。
/// - `n`: 最多检查的字符数（`size_t` 类型）。
///
/// **后置条件**:
/// - Case 1: 所有 `n` 个字符（或到终止 null）都可打印且列宽已知
///   -> 返回累计的列宽总和（非负整数 `c_int`）。
/// - Case 2: 遇到不可打印字符（`wcwidth` 返回 -1）
///   -> 提前终止，返回 -1。
///
/// **不变量**: 纯函数。不修改 `wcs` 指向的内容。线程安全。
///
/// # Safety
///
/// 调用者必须确保:
/// - `wcs` 为非 NULL 指针，指向以 null 结尾的有效宽字符串
/// - 若 `n` 大于字符串实际长度，读取将在 null 终止符处安全停止
///
/// [ISO C 标准库 `<wchar.h>`]
#[no_mangle]
pub extern "C" fn wcswidth(wcs: *const wchar_t, n: size_t) -> c_int {
    // musl 原实现:
    //   int l=0, k=0;
    //   for (; n-- && *wcs && (k = wcwidth(*wcs)) >= 0; l+=k, wcs++);
    //   return (k < 0) ? k : l;
    let mut total: c_int = 0;
    let mut remaining = n;

    unsafe {
        let mut p = wcs;
        while remaining > 0 && *p != 0 {
            let k = wcwidth(*p);
            if k < 0 {
                return -1;
            }
            total += k;
            p = p.add(1);
            remaining -= 1;
        }
    }
    total
}
