//! iswalnum —— 判断宽字符是否为字母或数字。
//! 对应 musl src/ctype/iswalnum.c
//!
//! 采用"数字优先检测"策略：先执行代价极低的 `iswdigit` 快速路径检查
//!（单次无符号范围比较），仅在该检查失败后才调用 `iswalpha` 进行位图查表。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::{wint_t, locale_t, WEOF};
use rusl_internal::libc::__locale_struct;

/// C 标准 iswalnum —— 判断宽字符是否为字母或数字。
///
/// [Visibility]: External —— POSIX.1-2001 标准函数，`<wctype.h>` 声明，ABI 兼容。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`（即 `c_uint`），任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。
///
/// # 返回值
///
/// * 若 `wc` 是十进制数字或 Unicode 字母，返回非零值（musl 中为 1）。
/// * 数字分支优先：`iswdigit(wc)` 为 true 时直接返回 1。
/// * 数字检查失败但 `iswalpha(wc)` 为 true 时，返回非零值。
/// * `wc` 既不是数字也不是字母，或 `wc == WEOF` 时返回 0。
///
/// # 行为说明
///
/// 采用"数字优先检测"策略：先执行代价极低的快速路径检查（单次无符号范围比较），
/// 仅在该检查失败后才调用 `iswalpha` 进行位图查表。
/// 纯函数，无副作用，无内部状态，完全线程安全。
/// 本实现不依赖 locale 设置。
///
/// # 算法
///
/// 对应 musl `src/ctype/iswalnum.c`:
/// ```c
/// if (iswdigit(wc)) return 1;
/// return iswalpha(wc);
/// ```
#[no_mangle]
pub extern "C" fn iswalnum(wc: wint_t) -> c_int {
    // Step 1: 数字快速路径 —— iswdigit 内联
    // iswdigit: wc.wrapping_sub('0') < 10
    if wc.wrapping_sub(b'0' as wint_t) < 10 {
        return 1;
    }
    // Step 2: 字母查表路径 —— 委托给 iswalpha
    super::iswalpha::iswalpha(wc)
}

/// iswalnum_l_impl —— iswalnum_l 的内部实现体（C 中对应 `__iswalnum_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，更名为 `iswalnum_l_impl` 并作为模块内部函数。
/// POSIX locale-aware 字符分类函数的内部实现桩。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
/// * `l` - 类型为 `locale_t`（`*mut __locale_struct`），指向有效的 locale 结构
///   或为 `null_mut()`（表示 C locale）。在 musl 中**被忽略**。
///
/// # 返回值
///
/// 完全等效于 `iswalnum(wc)` 的返回值。
#[inline]
pub(crate) fn iswalnum_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswalnum(wc)
}

/// iswalnum_l —— locale-aware 宽字符字母/数字判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `iswalnum` 完全一致。
/// Rust 中 `iswalnum_l` 为独立的 `extern "C"` 函数，内部委托给 `iswalnum_l_impl`
///（相当于 C 中 `weak_alias` 的效果）。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，同 `iswalnum`。
/// * `l` - 类型为 `locale_t`，在 musl 中被忽略。
///
/// # 返回值
///
/// 完全等效于 `iswalnum(wc)` 的返回值。
#[no_mangle]
pub extern "C" fn iswalnum_l(wc: wint_t, l: locale_t) -> c_int {
    iswalnum_l_impl(wc, l)
}