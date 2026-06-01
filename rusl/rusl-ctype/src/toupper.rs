//! toupper/toupper_l — 将小写字母转换为大写字母。
//! 对应 musl src/ctype/toupper.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_void};

// ============================================================================
// 内部实现
// ============================================================================

/// 核心转换逻辑：若 c 是小写字母 ('a'-'z')，返回 c & 0x5f；否则返回原值。
#[inline]
fn toupper_impl(c: c_int) -> c_int {
    // musl 原实现: if (islower(c)) return c & 0x5f; return c;
    // 此处内联 islower 逻辑: (unsigned)c - 'a' < 26
    if (c as u32).wrapping_sub(b'a' as u32) < 26 {
        c & 0x5f
    } else {
        c
    }
}

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 将小写字母转换为大写字母。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
///
/// **后置条件**:
/// - 若 `c` 是小写字母 (`'a'`-`'z'`)：返回对应的大写字母 (`c & 0x5f`)。
/// - 否则：返回 `c` 原值。
///
/// **不变量**: 纯函数（无副作用，仅依赖输入参数），线程安全（无共享可变状态）。
///
/// [ISO C 标准库 `<ctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn toupper(c: c_int) -> c_int {
    toupper_impl(c)
}

/// POSIX.1-2008: locale-aware 大写转换，行为与 toupper 相同（当前单 locale 实现）。
///
/// **前置条件**: `c` 必须可表示为 `unsigned char` 或等于 `EOF`。
/// `l` 为 locale 句柄（当前实现中忽略）。
///
/// **后置条件**: 与 `toupper(c)` 相同。
///
/// [POSIX 扩展 `<ctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn toupper_l(c: c_int, _l: *mut c_void) -> c_int {
    toupper_impl(c)
}

// ============================================================================
// 内部符号 (不对外导出)
// ============================================================================

/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 toupper。
///
/// 对应 C 的 `__toupper_l`。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __toupper_l(c: c_int, _l: *mut c_void) -> c_int {
    toupper_impl(c)
}
