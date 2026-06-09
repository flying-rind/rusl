//! iswdigit — 宽字符十进制数字判断。
//! 对应 musl src/ctype/iswdigit.c

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 判断宽字符是否为十进制数字字符 (U+0030..U+0039, 即 '0'..'9')。
///
/// 算法: 单行无符号区间检查 `(wc as u32).wrapping_sub('0' as u32) < 10`。
///
/// # 安全性
///
/// 实际调用无内存安全性风险。
#[no_mangle]
pub extern "C" fn iswdigit(wc: wint_t) -> c_int {
    __iswdigit_l(wc, core::ptr::null_mut())
}

/// locale 感知的十进制数字判断。
///
/// 在 C locale 下行为与 [`iswdigit`] 完全等价。
/// 某些 locale 可能包含非 ASCII 数字 (如阿拉伯数字 U+0660-U+0669)。
///
#[no_mangle]
pub extern "C" fn iswdigit_l(wc: wint_t, l: locale_t) -> c_int {
    __iswdigit_l(wc, l)
}

/// 内部实现函数。供 [`iswctype_l`] 等内部分类函数调用。
///
/// [`iswdigit`] 等价于 `__iswdigit_l(wc, core::ptr::null_mut())`。
///
/// 算法: 单行无符号区间检查 `(wc as u32).wrapping_sub('0' as u32) < 10`。
/// 当 wc < '0' 时 wrapping_sub 产生大值, 自然不满足 < 10 条件。
/// O(1) 时间复杂度, 无分支。
///
/// `_l` 参数当前保留为 API 兼容占位, 内部实现仅覆盖 ASCII 数字。
///
/// # 安全性
///
/// 同 [`iswdigit_l`]。
pub(crate) fn __iswdigit_l(wc: wint_t, _l: locale_t) -> c_int {
    // 无符号区间检查: '0'=0x30, '9'=0x39, 共 10 个码点
    (wc.wrapping_sub(0x30) < 10) as c_int
}
