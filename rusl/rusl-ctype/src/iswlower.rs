//! iswlower — 宽字符小写字母判断。
//! 对应 musl src/ctype/iswlower.c
//!
//! 通过检测 `towupper(wc) != wc` 反向推断: 字符是小写字母当且仅当它
//! 存在不同的大写形式。此策略避免维护独立的小写分类表。

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 判断宽字符是否为小写字母。
///
/// 算法: 通过 `towupper(wc) != wc` 判定。
/// - 小写字母 (如 'a'-'z'): `towupper` 返回对应大写, 不等于 `wc` -> 非零
/// - 大写字母 (如 'A'-'Z'): `towupper` 返回自身, 等于 `wc` -> 0
/// - 无大小写字符 (数字、标点等): `towupper` 返回自身 -> 0
/// - WEOF: `towupper(WEOF) == WEOF` -> 0
///
/// # 安全性
///
/// 此函数标记为 `unsafe` 以保持 C ABI 签名兼容。
/// 实际调用无内存安全性风险。
#[no_mangle]
pub unsafe extern "C" fn iswlower(wc: wint_t) -> c_int {
    __iswlower_l(wc, core::ptr::null_mut())
}

/// locale 感知的小写字母判断。
///
/// 等价于 `towupper_l(wc, l) != wc`。
/// 某些 locale 可能额外定义小写字母 (如带变音符号的字母)。
///
/// # 安全性
///
/// - `l`: 必须为有效的 locale 句柄, 或 `NULL` 表示 C locale。
#[no_mangle]
pub unsafe extern "C" fn iswlower_l(wc: wint_t, l: locale_t) -> c_int {
    __iswlower_l(wc, l)
}

/// 内部实现函数。供 [`iswctype_l`] 等内部分类函数调用。
///
/// [`iswlower`] 等价于 `__iswlower_l(wc, core::ptr::null_mut())`。
///
/// 算法: 通过 `towupper(wc) != wc` 判定。字符是小写字母当且仅当
/// 它存在不同的大写形式。此策略避免维护独立的小写分类表,
/// 自动与 `towupper` 的大小写映射保持一致。
///
/// 关键边界情况:
/// - 小写字母 `a-z`: `towupper('a') == 'A' != 'a'` -> 非零
/// - 大写字母 `A-Z`: `towupper('A') == 'A' == 'A'` -> 0
/// - 无大小写字符: `towupper('1') == '1'` -> 0
/// - WEOF: `towupper(WEOF) == WEOF` -> 0
///
/// `_l` 参数当前保留为 API 兼容占位。
///
/// # 安全性
///
/// 同 [`iswlower_l`]。
pub(crate) fn __iswlower_l(wc: wint_t, _l: locale_t) -> c_int {
    // 调用 towupper 获取大写映射, 比较是否与自身不同
    // 注意: towupper 为 unsafe extern "C", 但以值类型 wint_t 调用是安全的
    (unsafe { super::towupper(wc) } != wc) as c_int
}
