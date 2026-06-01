//! isdigit/isdigit_l — 判断字符是否为十进制数字。
//! 对应 musl src/ctype/isdigit.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::locale_t;

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 判断字符 c 是否为十进制数字字符（'0'-'9'）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
///
/// **后置条件**:
/// - 若 `c` 是十进制数字（'0' = 0x30 至 '9' = 0x39）: 返回非零值。
/// - 否则（包括 `EOF`）: 返回 0。
///
/// **不变量**: 纯函数，无内部状态。
///
/// [ISO C 标准库 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn isdigit(c: c_int) -> c_int {
    ((c as u32).wrapping_sub(b'0' as u32) < 10) as c_int
}

/// POSIX.1-2008: locale-aware 变体，行为与 isdigit 相同（当前单 locale 实现）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
/// `l` 为 locale 句柄（当前 C/POSIX locale 实现中忽略）。
///
/// **后置条件**: 与 `isdigit(c)` 相同。
///
/// [POSIX 扩展 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn isdigit_l(c: c_int, _l: locale_t) -> c_int {
    isdigit(c)
}

// ============================================================================
// 内部符号 (不对外导出)
// ============================================================================

/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 isdigit。
///
/// 对应 C 的 `__isdigit_l`，musl 中 isdigit_l 通过 weak_alias 指向此函数。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __isdigit_l(c: c_int, _l: locale_t) -> c_int {
    isdigit(c)
}
