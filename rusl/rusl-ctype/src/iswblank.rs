//! iswblank —— 判断宽字符是否为空白字符（空格或水平制表符）。
//! 对应 musl src/ctype/iswblank.c
//!
//! 与 `isspace` 不同，`iswblank` 仅识别空格和水平制表符（POSIX "blank" 字符类），
//! 不包含换行、垂直制表符等其他空白字符。
//!
//! 由于目标字符均在 ASCII 范围内且宽字符编码与 ASCII 同值，
//! musl 直接委托给 `isblank`，避免重复实现。
//! Rust 中继续保持此委托模式。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::{wint_t, locale_t, WEOF};
use crate::import::__locale_struct;

/// C 标准 iswblank —— 判断宽字符是否为空白字符（空格或水平制表符）。
///
/// [Visibility]: External —— POSIX.1-2001 标准函数，`<wctype.h>` 声明，ABI 兼容。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`（即 `c_uint`），任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。
///
/// # 返回值
///
/// * 若 `wc` 是空白字符（空格 `L' ' = 0x20` 或水平制表符 `L'\t' = 0x09`），
///   返回非零值（musl 中为 1）。
/// * 其他字符或 `WEOF` 返回 0。
///
/// # 行为说明
///
/// 纯函数，无副作用，无内部可变状态，完全线程安全。
/// 由于 ASCII 空格 (`0x20`) 和水平制表符 (`0x09`) 在宽字符编码中的值与 `char` 完全相同，
/// `iswblank(wc)` 内部逻辑与 `isblank(wc as c_int)` 等价。
/// 对于 `WEOF = 0xffffffff`，不等于 0x20 或 0x09，正确返回 0。
///
/// # 算法
///
/// 对应 musl `src/ctype/iswblank.c`: `return isblank(wc);`
/// 其中 isblank 实现为 `c == ' ' || c == '\t'`。
#[no_mangle]
pub extern "C" fn iswblank(wc: wint_t) -> c_int {
    // isblank 等价逻辑: 仅空格 (0x20) 和水平制表符 (0x09)
    (wc == 0x20 || wc == 0x09) as c_int
}

/// iswblank_l_impl —— iswblank_l 的内部实现体（C 中对应 `__iswblank_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，更名为 `iswblank_l_impl` 并作为模块内部函数。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
/// * `l` - 类型为 `locale_t`（`*mut __locale_struct`），指向有效的 locale 结构
///   或为 `null_mut()`（表示 C locale）。在 musl 中**被忽略**。
///
/// # 返回值
///
/// 完全等效于 `iswblank(wc)` 的返回值。
#[inline]
pub(crate) fn iswblank_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswblank(wc)
}

/// iswblank_l —— locale-aware 宽字符空白判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `iswblank` 完全一致。
/// Rust 中 `iswblank_l` 为独立的 `extern "C"` 函数，内部委托给 `iswblank_l_impl`
///（相当于 C 中 `weak_alias` 的效果）。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，同 `iswblank`。
/// * `l` - 类型为 `locale_t`，在 musl 中被忽略。
///
/// # 返回值
///
/// 完全等效于 `iswblank(wc)` 的返回值。
#[no_mangle]
pub extern "C" fn iswblank_l(wc: wint_t, l: locale_t) -> c_int {
    iswblank_l_impl(wc, l)
}