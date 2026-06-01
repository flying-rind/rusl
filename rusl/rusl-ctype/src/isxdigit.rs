//! isxdigit — 测试字符是否为十六进制数字字符。
//! 对应 musl src/ctype/isxdigit.c

use core::ffi::c_int;

use rusl_core::c_types::locale_t;

/// 测试字符 c 是否为十六进制数字字符（'0'-'9'、'A'-'F' 或 'a'-'f'）。
///
/// # 参数
///
/// * `c` - 字符值，类型为 `c_int`，值必须可表示为 `unsigned char` 或等于 `EOF` (-1)
///
/// # 返回
///
/// * 如果 c 是十六进制数字字符，返回非零值
/// * 否则返回 0
///
/// # Safety
///
/// 此函数标记为 unsafe 以匹配 C ABI 调用约定。
#[no_mangle]
pub unsafe extern "C" fn isxdigit(c: c_int) -> c_int {
    __isxdigit_l(c)
}

/// 测试字符 c 是否为十六进制数字字符（带 locale 参数，当前未使用）。
///
/// `locale` 参数保留为 API 兼容占位，内部实现忽略此参数。
/// 行为与 `isxdigit(c)` 完全一致。
///
/// # Safety
///
/// `l` 若不为 null 则必须指向有效的 locale 对象。
#[no_mangle]
pub unsafe extern "C" fn isxdigit_l(c: c_int, _l: locale_t) -> c_int {
    __isxdigit_l(c)
}

/// isxdigit 和 isxdigit_l 的内部委托实现。
///
/// 使用无符号减法区间检查：
/// - 数字区间: `c.wrapping_sub(b'0') < 10`
/// - 字母区间: `(c | 32).wrapping_sub(b'a') < 6`（通过 `|32` 统一大小写）
///
/// 与宽字符版本 `__iswxdigit_l` 的逻辑完全一致，仅输入类型从 `wint_t` 变为 `c_int`。
pub(crate) fn __isxdigit_l(c: c_int) -> c_int {
    let u = c as u8;
    let is_digit = u.wrapping_sub(b'0') < 10;
    let is_alpha = (u | 32).wrapping_sub(b'a') < 6;
    (is_digit || is_alpha) as c_int
}
