//! iscntrl/iscntrl_l — 判断字符是否为控制字符。
//! 对应 musl src/ctype/iscntrl.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::locale_t;

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 判断字符 c 是否为控制字符（0x00-0x1F 或 0x7F）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
///
/// **后置条件**:
/// - 若 `c` 是控制字符（C0 控制字符 0x00-0x1F 或 DEL 0x7F）: 返回非零值。
/// - 否则（包括 `EOF`）: 返回 0。
///
/// **不变量**: 纯函数，线程安全。
///
/// [ISO C 标准库 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn iscntrl(c: c_int) -> c_int {
    ((c as u32) < 0x20 || c == 0x7f) as c_int
}

/// POSIX.1-2008: locale-aware 变体，行为与 iscntrl 相同（当前单 locale 实现）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
/// `l` 为 locale 句柄（当前 C/POSIX locale 实现中忽略）。
///
/// **后置条件**: 与 `iscntrl(c)` 相同。
///
/// [POSIX 扩展 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn iscntrl_l(c: c_int, _l: locale_t) -> c_int {
    iscntrl(c)
}

// ============================================================================
// 内部符号 (不对外导出)
// ============================================================================

/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 iscntrl。
///
/// 对应 C 的 `__iscntrl_l`，musl 中 iscntrl_l 通过 weak_alias 指向此函数。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __iscntrl_l(c: c_int, _l: locale_t) -> c_int {
    iscntrl(c)
}
