//! iswspace — 测试宽字符是否为 Unicode 空白字符。
//! 对应 musl src/ctype/iswspace.c
//!
//! 判断宽字符是否为 Unicode White_Space 属性的空白字符。
//! 排除了不间断空格（U+00A0, U+2007, U+202F）和非空白字形的脚本特定字符（U+1680, U+180E）。

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// SPACES 静态只读常量数组: 21 个 Unicode 空白字符码点。
///
/// 来源: musl `iswspace.c` 中的 `spaces` 数组（不含末尾终止符 0）。
const SPACES: [wint_t; 21] = [
    0x0020, // ' '
    0x0009, // '\t'
    0x000A, // '\n'
    0x000D, // '\r'
    0x000B, // '\v'
    0x000C, // '\f'
    0x0085, // NEL (Next Line)
    0x2000, // En Quad
    0x2001, // Em Quad
    0x2002, // En Space
    0x2003, // Em Space
    0x2004, // Three-Per-Em Space
    0x2005, // Four-Per-Em Space
    0x2006, // Six-Per-Em Space
    0x2008, // Punctuation Space
    0x2009, // Thin Space
    0x200A, // Hair Space
    0x2028, // Line Separator
    0x2029, // Paragraph Separator
    0x205F, // Medium Mathematical Space
    0x3000, // Ideographic Space
];

/// 测试宽字符 wc 是否为 Unicode 空白字符。
///
/// 空白字符列表（共 21 个码点）：
/// `' '`, `'\t'`, `'\n'`, `'\r'`, `'\v'`, `'\f'`,
/// U+0085, U+2000-U+2006, U+2008-U+200A, U+2028, U+2029, U+205F, U+3000
///
/// 注意: musl C 源码中 SPACES 数组为 22 个元素（含末尾终止符 0），
/// Rust 实现不含终止符，实际空白字符为 21 个。
///
/// # 参数
///
/// * `wc` - 宽字符值，类型为 `wint_t` (`c_uint`)，任意宽字符值（含 `WEOF`）
///
/// # 返回
///
/// * 如果 wc 是空白字符，返回非零值
/// * 如果 wc 不是空白字符或 wc == 0，返回 0
///
/// # 特殊处理
///
/// `wc == 0` 直接返回 0，防止搜索函数将空字符误匹配到 SPACES 数组的终止符。
///
/// # Safety
///
/// 此函数标记为 unsafe 以匹配 C ABI 调用约定。
#[no_mangle]
pub unsafe extern "C" fn iswspace(wc: wint_t) -> c_int {
    __iswspace_l(wc) as c_int
}

/// 测试宽字符 c 是否为 Unicode 空白字符（带 locale 参数，当前未使用）。
///
/// `locale` 参数保留为 API 兼容占位，内部实现忽略此参数。
/// 行为与 `iswspace(wc)` 完全一致。
///
/// # Safety
///
/// `l` 若不为 null 则必须指向有效的 locale 对象。
#[no_mangle]
pub unsafe extern "C" fn iswspace_l(c: wint_t, _l: locale_t) -> c_int {
    __iswspace_l(c) as c_int
}

/// iswspace 和 iswspace_l 的内部委托实现。
///
/// 在预定义的 SPACES 数组（21 个空白字符码点）中线性搜索 wc。
/// `wc == 0` 直接返回 false，防止线性搜索将 `'\0'` 误匹配到数组终止符。
pub(crate) fn __iswspace_l(c: wint_t) -> bool {
    // wc == 0 直接返回 false，防止线性搜索将 '\0' 误匹配。
    c != 0 && SPACES.contains(&c)
}
