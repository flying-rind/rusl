//! wctrans / towctrans —— 宽字符大小写变换描述符。
//! 对应 musl src/ctype/wctrans.c
//!
//! 提供变换名称字符串到变换描述符的映射，以及基于描述符的字符变换执行。
//! 变换描述符为固定整数值: 0 (无效)、1 (toupper)、2 (tolower)。

use core::ffi::{c_char, c_int, c_void};

use rusl_core::c_types::{wctrans_t, wint_t};

// 引用同模块的 towlower/towupper 函数
use super::towctrans::{towlower, towupper};

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 对以 null 结尾的 C 字符串执行与 "toupper" / "tolower" 的快速字节比较。
///
/// C 原实现使用 strcmp，此处内联字节比较以避免对外部 strcmp 的运行时依赖。
///
/// # Safety
///
/// 调用者必须确保 `s` 为非 NULL 指针，指向以 null 结尾的有效 C 字符串。
unsafe fn cstr_cmp(s: *const c_char, literal: &[u8]) -> bool {
    let mut p = s;
    let mut i = 0;
    loop {
        let byte = *p as u8;
        if byte == 0 && i == literal.len() {
            return true;
        }
        if i >= literal.len() {
            return false;
        }
        if byte != literal[i] {
            return false;
        }
        i += 1;
        p = p.add(1);
    }
}

// ============================================================================
// 公开导出接口 (C ABI 兼容)
// ============================================================================

/// ISO C 标准库: 将变换名称字符串解析为变换描述符。
///
/// 支持的变换名称:
/// - `"toupper"` -> 返回 1
/// - `"tolower"` -> 返回 2
/// - 其他 -> 返回 0（无效描述符）
///
/// **前置条件**:
/// - `class`: 指向以 null 结尾的 C 字符串，内容为 `"toupper"` 或 `"tolower"`。
///   若 `class` 为 NULL，行为未定义。
///
/// **后置条件**:
/// - `class == "toupper"` -> 返回 `(wctrans_t)1`
/// - `class == "tolower"` -> 返回 `(wctrans_t)2`
/// - 其他字符串 -> 返回 0
///
/// **不变量**: 纯函数。变换描述符是固定整数，不与任何动态资源关联。线程安全。
///
/// # Safety
///
/// 调用者必须确保:
/// - `class` 为非 NULL 指针，指向以 null 结尾的有效 C 字符串
///
/// [ISO C 标准库 `<wctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn wctrans(class: *const c_char) -> wctrans_t {
    // musl 原实现:
    //   if (!strcmp(class, "toupper")) return (wctrans_t)1;
    //   if (!strcmp(class, "tolower")) return (wctrans_t)2;
    //   return 0;
    unsafe {
        if cstr_cmp(class, b"toupper") {
            return 1;
        }
        if cstr_cmp(class, b"tolower") {
            return 2;
        }
    }
    0
}

/// ISO C 标准库: 根据变换描述符执行大小写变换。
///
/// - `trans == 1`（"toupper"）-> 调用 `towupper(wc)` 并返回结果
/// - `trans == 2`（"tolower"）-> 调用 `towlower(wc)` 并返回结果
/// - 其他 `trans` 值 -> 返回 `wc` 原值
///
/// **前置条件**:
/// - `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
/// - `trans`: 由 `wctrans()` 返回的变换描述符。
///
/// **后置条件**:
/// - `trans == 1` -> 返回 `towupper(wc)` 的结果
/// - `trans == 2` -> 返回 `towlower(wc)` 的结果
/// - `trans` 为其他值 -> 返回 `wc`
///
/// **不变量**: 纯函数。线程安全。
///
/// [ISO C 标准库 `<wctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn towctrans(wc: wint_t, trans: wctrans_t) -> wint_t {
    // musl 原实现:
    //   if (trans == (wctrans_t)1) return towupper(wc);
    //   if (trans == (wctrans_t)2) return towlower(wc);
    //   return wc;
    match trans {
        1 => unsafe { towupper(wc) },
        2 => unsafe { towlower(wc) },
        _ => wc,
    }
}

/// POSIX.1-2008: locale-aware 变换名解析。
///
/// 忽略 `locale_t` 参数（在当前单 locale 实现中），
/// 行为与 `wctrans(class)` 完全相同。
///
/// # Safety
///
/// 调用者必须确保:
/// - `class` 为非 NULL 指针，指向以 null 结尾的有效 C 字符串
/// - `l`: 必须为有效的 locale 句柄，或 `NULL` 表示 C locale
///
/// [POSIX 扩展 `<wctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn wctrans_l(
    class: *const c_char,
    _l: *mut c_void,
) -> wctrans_t {
    unsafe { wctrans(class) }
}

/// POSIX.1-2008: locale-aware 变换执行。
///
/// 忽略 `locale_t` 参数（在当前单 locale 实现中），
/// 行为与 `towctrans(wc, trans)` 完全相同。
///
/// # Safety
///
/// - `l`: 必须为有效的 locale 句柄，或 `NULL` 表示 C locale
///
/// [POSIX 扩展 `<wctype.h>`]
#[no_mangle]
pub unsafe extern "C" fn towctrans_l(
    wc: wint_t,
    trans: wctrans_t,
    _l: *mut c_void,
) -> wint_t {
    unsafe { towctrans(wc, trans) }
}

// ============================================================================
// 内部符号 (不对外导出)
// ============================================================================

/// 内部 locale-aware 变换名解析实现，忽略 locale 参数，直接委托 wctrans。
///
/// 对应 C 的 `__wctrans_l`。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __wctrans_l(class: *const c_char, _l: *mut c_void) -> wctrans_t {
    // 安全: 直接委托给 wctrans，由调用者保证 class 有效性
    unsafe { wctrans(class) }
}

/// 内部 locale-aware 变换执行实现，忽略 locale 参数，直接委托 towctrans。
///
/// 对应 C 的 `__towctrans_l`。
///
/// Rust 设计: 使用 `pub` 可见性以便集成测试访问，
/// 但不使用 `#[no_mangle]`，不会作为 C ABI 符号导出。
pub fn __towctrans_l(wc: wint_t, trans: wctrans_t, _l: *mut c_void) -> wint_t {
    unsafe { towctrans(wc, trans) }
}
