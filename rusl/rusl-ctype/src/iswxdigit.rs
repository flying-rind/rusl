//! iswxdigit — 测试宽字符是否为十六进制数字字符。
//! 对应 musl src/ctype/iswxdigit.c

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 测试宽字符 wc 是否为十六进制数字字符（'0'-'9'、'A'-'F' 或 'a'-'f'）。
///
/// # 参数
///
/// * `wc` - 宽字符值，类型为 `wint_t` (`c_uint`)，任意宽字符值（含 `WEOF`）
///
/// # 返回
///
/// * 如果 wc 是十六进制数字字符，返回非零值
/// * 否则返回 0
///
/// # Safety
///
/// 此函数标记为 unsafe 以匹配 C ABI 调用约定。
#[no_mangle]
pub unsafe extern "C" fn iswxdigit(wc: wint_t) -> c_int {
    __iswxdigit_l(wc)
}

/// 测试宽字符 c 是否为十六进制数字字符（带 locale 参数，当前未使用）。
///
/// `locale` 参数保留为 API 兼容占位，内部实现忽略此参数。
/// 行为与 `iswxdigit(wc)` 完全一致。
///
/// # Safety
///
/// `l` 若不为 null 则必须指向有效的 locale 对象。
#[no_mangle]
pub unsafe extern "C" fn iswxdigit_l(c: wint_t, _l: locale_t) -> c_int {
    __iswxdigit_l(c)
}

/// iswxdigit 和 iswxdigit_l 的内部委托实现。
///
/// 使用两个无符号区间检查：
/// - 数字区间: `c.wrapping_sub('0' as u32) < 10`
/// - 字母区间: `(c | 32).wrapping_sub('a' as u32) < 6`（通过 `|32` 统一大小写）
///
/// 与 `isxdigit` 的 `__isxdigit_l` 逻辑完全一致，仅输入类型从 `c_int` 变为 `wint_t`。
pub(crate) fn __iswxdigit_l(c: wint_t) -> c_int {
    let is_digit = c.wrapping_sub(b'0' as wint_t) < 10;
    let is_alpha = (c | 32).wrapping_sub(b'a' as wint_t) < 6;
    (is_digit || is_alpha) as c_int
}
