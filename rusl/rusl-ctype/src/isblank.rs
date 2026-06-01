//! isblank/isblank_l — 判断字符是否为空白字符（空格或水平制表符）。
//! 对应 musl src/ctype/isblank.c

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::locale_t;

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 判断字符 c 是否为空白字符（空格 0x20 或水平制表符 0x09）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
///
/// **后置条件**:
/// - 若 `c` 是空格 (`' '` = 0x20) 或水平制表符 (`'\t'` = 0x09): 返回非零值。
/// - 否则（包括 `EOF`）: 返回 0。
///
/// **不变量**: 纯函数，线程安全，不依赖 locale 状态。
///
/// [ISO C 标准库 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn isblank(c: c_int) -> c_int {
    (c == 0x20 || c == 0x09) as c_int
}

/// POSIX.1-2008: locale-aware 变体，行为与 isblank 相同（当前单 locale 实现）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
/// `l` 为 locale 句柄（当前 C/POSIX locale 实现中忽略）。
///
/// **后置条件**: 与 `isblank(c)` 相同。
///
/// [POSIX 扩展 `<ctype.h>`]
#[no_mangle]
pub extern "C" fn isblank_l(c: c_int, _l: locale_t) -> c_int {
    isblank(c)
}

// ============================================================================
// 内部符号 (不对外导出)
// ============================================================================

/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 isblank。
///
/// 对应 C 的 `__isblank_l`，musl 中 isblank_l 通过 weak_alias 指向此函数。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __isblank_l(c: c_int, _l: locale_t) -> c_int {
    isblank(c)
}
