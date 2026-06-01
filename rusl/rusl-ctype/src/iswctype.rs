//! iswctype / wctype — 通用宽字符分类与分类名称解析。
//! 对应 musl src/ctype/iswctype.c
//!
//! 此模块实现 POSIX `<wctype.h>` 的可扩展字符分类机制:
//! - `wctype()` 将分类名称字符串解析为分类标识符
//! - `iswctype()` 根据分类标识符判断字符是否属于该分类

use core::ffi::{c_char, c_int};

use rusl_core::c_types::{locale_t, wctype_t, wint_t};

// ============================================================================
// 分类标识符常量
// ============================================================================

/// 字母数字分类标识符 (alnum = alpha | digit)
pub const WCTYPE_ALNUM: wctype_t = 1;
/// 字母分类标识符
pub const WCTYPE_ALPHA: wctype_t = 2;
/// 空白字符分类标识符 (空格 + 制表符)
pub const WCTYPE_BLANK: wctype_t = 3;
/// 控制字符分类标识符
pub const WCTYPE_CNTRL: wctype_t = 4;
/// 十进制数字分类标识符
pub const WCTYPE_DIGIT: wctype_t = 5;
/// 图形字符分类标识符 (打印字符排除空格)
pub const WCTYPE_GRAPH: wctype_t = 6;
/// 小写字母分类标识符
pub const WCTYPE_LOWER: wctype_t = 7;
/// 可打印字符分类标识符
pub const WCTYPE_PRINT: wctype_t = 8;
/// 标点符号分类标识符
pub const WCTYPE_PUNCT: wctype_t = 9;
/// 空白字符分类标识符 (空格 + 换行等)
pub const WCTYPE_SPACE: wctype_t = 10;
/// 大写字母分类标识符
pub const WCTYPE_UPPER: wctype_t = 11;
/// 十六进制数字分类标识符
pub const WCTYPE_XDIGIT: wctype_t = 12;

// ============================================================================
// 公共 API
// ============================================================================

/// 通用宽字符分类函数。
///
/// 根据 `desc` (由 [`wctype`] 返回的分类标识符) 判断 `wc` 是否属于对应分类。
///
/// - 若 `desc` 匹配某个已知分类且 `wc` 属于该分类 -> 返回非零值
/// - 若 `desc` 无效或 `wc` 不属于该分类 -> 返回 0
///
/// 等价于 `__iswctype_l(wc, desc, core::ptr::null_mut())`。
///
/// # 安全性
///
/// - `desc`: 应由 [`wctype`] 返回。传入任意 `wctype_t` 值是安全的,
///   但无效值会导致返回 0 (行为实现定义)。
#[no_mangle]
pub unsafe extern "C" fn iswctype(wc: wint_t, desc: wctype_t) -> c_int {
    __iswctype_l(wc, desc, core::ptr::null_mut())
}

/// 将分类名称字符串解析为分类标识符。
///
/// 支持的分类名称 (与 WCTYPE_* 常量顺序严格对应):
/// `"alnum"`, `"alpha"`, `"blank"`, `"cntrl"`, `"digit"`,
/// `"graph"`, `"lower"`, `"print"`, `"punct"`, `"space"`,
/// `"upper"`, `"xdigit"`
///
/// - 若 `name` 匹配已知分类 -> 返回对应标识符 (1-12)
/// - 若 `name` 不匹配 -> 返回 0
///
/// # 安全性
///
/// - `name`: 必须指向有效的以 null 结尾的 C 字符串。
///   传入 `NULL` 将导致未定义行为。
#[no_mangle]
pub unsafe extern "C" fn wctype(name: *const c_char) -> wctype_t {
    __wctype_l(name, core::ptr::null_mut())
}

/// locale 感知的通用宽字符分类。
///
/// 在 C locale 下行为与 [`iswctype`] 等价。
///
/// # 安全性
///
/// - `l`: 必须为有效的 locale 句柄, 或 `NULL` 表示 C locale。
#[no_mangle]
pub unsafe extern "C" fn iswctype_l(
    wc: wint_t,
    desc: wctype_t,
    l: locale_t,
) -> c_int {
    __iswctype_l(wc, desc, l)
}

/// locale 感知的分类名称解析。
///
/// # 安全性
///
/// - `name`: 必须指向有效的以 null 结尾的 C 字符串。
/// - `l`: 必须为有效的 locale 句柄, 或 `NULL` 表示 C locale。
#[no_mangle]
pub unsafe extern "C" fn wctype_l(name: *const c_char, l: locale_t) -> wctype_t {
    __wctype_l(name, l)
}

// ============================================================================
// 分类名称常量表
// ============================================================================

/// 固定的分类名称列表, 与 WCTYPE_* 常量顺序严格对应。
/// 用于 `wctype()` 的名称匹配。
/// 每个条目为 null 终止的字节串, 索引+1 即为对应的 WCTYPE_* 常量值。
const CLASS_NAMES: [&[u8]; 12] = [
    b"alnum\0",
    b"alpha\0",
    b"blank\0",
    b"cntrl\0",
    b"digit\0",
    b"graph\0",
    b"lower\0",
    b"print\0",
    b"punct\0",
    b"space\0",
    b"upper\0",
    b"xdigit\0",
];

// ============================================================================
// 内部接口
// ============================================================================

/// 内部实现函数。供其他内部分类函数复用。
///
/// [`iswctype`] 等价于 `__iswctype_l(wc, desc, core::ptr::null_mut())`。
///
/// 通过 match 语句分发到对应的 `isw*` 函数, 编译器生成跳转表优化。
/// `desc` 值 1-12 对应 WCTYPE_* 常量, 无效值返回 0。
///
/// `_l` 参数当前保留为 API 兼容占位。
pub(crate) fn __iswctype_l(
    wc: wint_t,
    desc: wctype_t,
    _l: locale_t,
) -> c_int {
    // SAFETY: 所有 isw* 函数的 unsafe 标记仅用于 C ABI 兼容,
    // 以值类型 wint_t 调用无内存安全性风险
    match desc {
        1 => super::iswalnum(wc),   // WCTYPE_ALNUM
        2 => super::iswalpha(wc),   // WCTYPE_ALPHA
        3 => super::iswblank(wc),   // WCTYPE_BLANK
        4 => unsafe { super::iswcntrl(wc) },   // WCTYPE_CNTRL
        5 => unsafe { super::iswdigit(wc) },   // WCTYPE_DIGIT
        6 => unsafe { super::iswgraph(wc) },   // WCTYPE_GRAPH
        7 => unsafe { super::iswlower(wc) },   // WCTYPE_LOWER
        8 => unsafe { super::iswprint(wc) },   // WCTYPE_PRINT
        9 => unsafe { super::iswpunct(wc) },   // WCTYPE_PUNCT
        10 => unsafe { super::iswspace(wc) },  // WCTYPE_SPACE
        11 => unsafe { super::iswupper(wc) },  // WCTYPE_UPPER
        12 => unsafe { super::iswxdigit(wc) }, // WCTYPE_XDIGIT
        _ => 0,                                // 无效标识符
    }
}

/// 内部实现函数。供其他内部模块复用。
///
/// [`wctype`] 等价于 `__wctype_l(name, core::ptr::null_mut())`。
///
/// 遍历固定的分类名称列表 (12 个条目), 逐字节比较。
/// 分类名称列表与 WCTYPE_* 常量顺序严格对应:
/// `CLASS_NAMES[i]` 对应标识符 `i+1`。
///
/// O(n) 时间复杂度, n=12, 每个条目最长为 7 字节 (含 null)。
///
/// # 安全性
///
/// - `name`: 调用者确保为有效的 null 终止 C 字符串。
///   传入 NULL 或不以 null 结尾的指针将导致未定义行为。
pub(crate) fn __wctype_l(name: *const c_char, _l: locale_t) -> wctype_t {
    for (idx, entry) in CLASS_NAMES.iter().enumerate() {
        let mut i: isize = 0;
        // SAFETY: 调用者确保 `name` 指向有效的 null 终止字符串
        let matched = unsafe {
            loop {
                // 检查边界: entry 已结束时, 检查 name 是否也结束
                if i as usize >= entry.len() {
                    break *name.offset(i) == 0;
                }
                let c = *name.offset(i);
                let e = entry[i as usize];
                // name 已结束 (遇到 null)
                if c == 0 {
                    // 两者同时结束才算匹配
                    break e == 0;
                }
                // 逐字节比较
                if (c as u8) != e {
                    break false;
                }
                i += 1;
            }
        };
        if matched {
            return (idx + 1) as wctype_t;
        }
    }
    0
}
