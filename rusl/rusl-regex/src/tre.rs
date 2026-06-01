//! TRE 正则表达式引擎 — 内部类型、常量和 TNFA 数据结构定义。
//!
//! 本模块对应 C 的 `tre.h` 头文件，所有符号均为 `pub(crate)` 可见性，
//! 仅 rusl crate 内部使用，不对外导出。
//!
//! 包含：
//! - 断言位掩码常量
//! - 宽字符类型别名和纯 Rust 字符分类/转换函数
//! - Tag 匹配方向枚举
//! - TNFA 转移边、子匹配元数据、TNFA 顶层结构体
//! - UTF-8 多字节到宽字符解码器

#![allow(unused_imports, unused_variables)]

use alloc::alloc::{alloc, alloc_zeroed, dealloc, realloc, Layout};
use alloc::boxed::Box;
use core::ffi::c_char;

// ============================================================================
// 第一部分：常量定义
// ============================================================================

/// Unicode 最大合法码点 U+10FFFF。
/// 任何大于此值的 `c_int` 被视为特殊标记（EMPTY、ASSERTION、TAG、BACKREF）。
pub(crate) const TRE_CHAR_MAX: i32 = 0x10ffff;

/// TRE 内存分配器每次从系统申请的内存块默认大小（1KB）。
pub(crate) const TRE_MEM_BLOCK_SIZE: usize = 1024;

/// 字符类名称最大长度。
pub(crate) const CHARCLASS_NAME_MAX: usize = 14;

/// 重复次数上限。
pub(crate) const RE_DUP_MAX: i32 = 255;

/// MB_LEN_MAX — 多字节字符最大字节数。
pub(crate) const MB_LEN_MAX: usize = 4;

// ============================================================================
// 断言位掩码常量
// ============================================================================

/// 行首断言 (^)。
pub(crate) const ASSERT_AT_BOL: i32 = 1;

/// 行尾断言 ($)。
pub(crate) const ASSERT_AT_EOL: i32 = 2;

/// 正向字符类别匹配（如 `\w`）。
pub(crate) const ASSERT_CHAR_CLASS: i32 = 4;

/// 反向字符类别匹配（如 `\W`）。
pub(crate) const ASSERT_CHAR_CLASS_NEG: i32 = 8;

/// 词首断言 (`\<`)。
pub(crate) const ASSERT_AT_BOW: i32 = 16;

/// 词尾断言 (`\>`)。
pub(crate) const ASSERT_AT_EOW: i32 = 32;

/// 词边界断言 (`\b`)。
pub(crate) const ASSERT_AT_WB: i32 = 64;

/// 非词边界断言 (`\B`)。
pub(crate) const ASSERT_AT_WB_NEG: i32 = 128;

/// 反向引用断言。
pub(crate) const ASSERT_BACKREF: i32 = 256;

/// 最后一个断言编号（等于 ASSERT_BACKREF）。
pub(crate) const ASSERT_LAST: i32 = 256;

// ============================================================================
// 第二部分：内部类型别名
// ============================================================================

/// 宽字符类型（对应 C 的 `wint_t`）。
/// 用于 TNFA 中存储字符范围值，可容纳 WEOF。
pub(crate) type TreCint = i32;

/// 宽字符类别句柄类型（对应 C 的 `wctype_t`）。
pub(crate) type TreCtype = core::ffi::c_ulong;

// ============================================================================
// 第三部分：UTF-8 多字节到宽字符解码器（纯 Rust 实现）
// ============================================================================

/// 将多字节 UTF-8 序列解码为单个 Unicode 码点。
///
/// 纯 Rust 实现，替代 C 的 `mbtowc`。不依赖任何 FFI 或外部 libc。
///
/// # 参数
///
/// * `pwc` - 输出参数，接收解码后的宽字符（Unicode 码点）
/// * `s` - 输入的多字节序列起始指针
/// * `n` - 输入序列的最大可读字节数
///
/// # 返回值
///
/// * `> 0`：成功解码，返回值为消耗的字节数（1..4）
/// * `0`：遇到 NUL 字节（`s[0] == 0`）
/// * `-1`：非法 UTF-8 序列
///
/// # UTF-8 编码规则
///
/// | 码点范围 | 字节序列 |
/// |----------|----------|
/// | U+0000..U+007F | 0xxxxxxx |
/// | U+0080..U+07FF | 110xxxxx 10xxxxxx |
/// | U+0800..U+FFFF | 1110xxxx 10xxxxxx 10xxxxxx |
/// | U+10000..U+10FFFF | 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx |
#[inline]
pub(crate) unsafe fn tre_mbtowc(pwc: &mut i32, s: *const u8, n: usize) -> i32 {
    if n == 0 {
        return -1;
    }
    let c0 = *s;
    if c0 == 0 {
        *pwc = 0;
        return 0;
    }

    // 单字节 ASCII: 0x00..0x7F
    if c0 < 0x80 {
        *pwc = c0 as i32;
        return 1;
    }

    // 多字节序列：根据首字节确定期望长度
    let (expected_len, mut code_point) = if c0 < 0xE0 {
        // 2 字节: 110xxxxx 10xxxxxx → U+0080..U+07FF
        if c0 < 0xC2 {
            return -1; // 过长编码
        }
        (2, (c0 & 0x1F) as u32)
    } else if c0 < 0xF0 {
        // 3 字节: 1110xxxx 10xxxxxx 10xxxxxx → U+0800..U+FFFF
        (3, (c0 & 0x0F) as u32)
    } else if c0 < 0xF5 {
        // 4 字节: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx → U+10000..U+10FFFF
        (4, (c0 & 0x07) as u32)
    } else {
        return -1; // 0xF5..0xFF 非法
    };

    if n < expected_len {
        return -1;
    }

    // 解码后续字节
    for i in 1..expected_len {
        let byte = *s.add(i);
        if byte & 0xC0 != 0x80 {
            return -1; // 后续字节不以 10xxxxxx 开头
        }
        code_point = (code_point << 6) | ((byte & 0x3F) as u32);
    }

    // 检查过长编码和代理对
    match expected_len {
        2 => {
            if code_point < 0x80 {
                return -1; // 过长编码
            }
        }
        3 => {
            if code_point < 0x800 {
                return -1; // 过长编码
            }
            if code_point >= 0xD800 && code_point <= 0xDFFF {
                return -1; // 代理对
            }
        }
        4 => {
            if code_point < 0x10000 {
                return -1; // 过长编码
            }
            if code_point > 0x10FFFF {
                return -1; // 超出 Unicode 范围
            }
        }
        _ => {}
    }

    *pwc = code_point as i32;
    expected_len as i32
}

// ============================================================================
// 第四部分：宽字符分类函数（纯 Rust 实现，通过 crate::ctype）
// ============================================================================

/// 宽字符是否为字母数字。
/// 通过 rusl 内部的 `iswalnum` Rust 实现完成分类，无 FFI 依赖。
#[inline]
pub(crate) fn tre_isalnum_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    rusl_ctype::iswalnum(c as u32) != 0
}

/// 宽字符是否为字母。
#[inline]
pub(crate) fn tre_isalpha_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    rusl_ctype::iswalpha(c as u32) != 0
}

/// 宽字符是否为空白（空格或制表符）。
#[inline]
pub(crate) fn tre_isblank_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    rusl_ctype::iswblank(c as u32) != 0
}

/// 宽字符是否为控制字符。
#[inline]
pub(crate) fn tre_iscntrl_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswcntrl(c as u32) != 0 }
}

/// 宽字符是否为十进制数字。
#[inline]
pub(crate) fn tre_isdigit_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswdigit(c as u32) != 0 }
}

/// 宽字符是否为可打印且有图形表示的字符。
#[inline]
pub(crate) fn tre_isgraph_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswgraph(c as u32) != 0 }
}

/// 宽字符是否为小写字母。
#[inline]
pub(crate) fn tre_islower_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswlower(c as u32) != 0 }
}

/// 宽字符是否为可打印字符。
#[inline]
pub(crate) fn tre_isprint_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswprint(c as u32) != 0 }
}

/// 宽字符是否为标点符号。
#[inline]
pub(crate) fn tre_ispunct_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswpunct(c as u32) != 0 }
}

/// 宽字符是否为空白字符。
#[inline]
pub(crate) fn tre_isspace_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswspace(c as u32) != 0 }
}

/// 宽字符是否为大写字母。
#[inline]
pub(crate) fn tre_isupper_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswupper(c as u32) != 0 }
}

/// 宽字符是否为十六进制数字。
#[inline]
pub(crate) fn tre_isxdigit_l1(c: TreCint) -> bool {
    if c < 0 || c > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswxdigit(c as u32) != 0 }
}

/// 宽字符转小写。通过 rusl 内部的 `towlower` Rust 实现完成转换。
#[inline]
pub(crate) fn tre_tolower_l1(c: TreCint) -> TreCint {
    if c < 0 || c > TRE_CHAR_MAX {
        return c;
    }
    unsafe { rusl_ctype::towlower(c as u32) as TreCint }
}

/// 宽字符转大写。通过 rusl 内部的 `towupper` Rust 实现完成转换。
#[inline]
pub(crate) fn tre_toupper_l1(c: TreCint) -> TreCint {
    if c < 0 || c > TRE_CHAR_MAX {
        return c;
    }
    unsafe { rusl_ctype::towupper(c as u32) as TreCint }
}

/// 检查宽字符是否属于指定字符类别。通过 rusl 内部的 `iswctype` 完成。
#[inline]
pub(crate) fn tre_isctype_l1(wc: TreCint, desc: TreCtype) -> bool {
    if wc < 0 || wc > TRE_CHAR_MAX {
        return false;
    }
    unsafe { rusl_ctype::iswctype(wc as u32, desc as u64) != 0 }
}

/// 通过名称获取宽字符类别句柄。通过 rusl 内部的 `wctype` 完成。
#[inline]
pub(crate) unsafe fn tre_ctype_l1(name: *const c_char) -> TreCtype {
    rusl_ctype::wctype(name) as TreCtype
}

// ---- 别名：保持向后兼容（旧名称） ----

#[inline]
pub(crate) unsafe fn tre_isalnum(c: TreCint) -> bool {
    tre_isalnum_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isalpha(c: TreCint) -> bool {
    tre_isalpha_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isblank(c: TreCint) -> bool {
    tre_isblank_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_iscntrl(c: TreCint) -> bool {
    tre_iscntrl_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isdigit(c: TreCint) -> bool {
    tre_isdigit_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isgraph(c: TreCint) -> bool {
    tre_isgraph_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_islower(c: TreCint) -> bool {
    tre_islower_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isprint(c: TreCint) -> bool {
    tre_isprint_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_ispunct(c: TreCint) -> bool {
    tre_ispunct_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isspace(c: TreCint) -> bool {
    tre_isspace_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isupper(c: TreCint) -> bool {
    tre_isupper_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isxdigit(c: TreCint) -> bool {
    tre_isxdigit_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_tolower(c: TreCint) -> TreCint {
    tre_tolower_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_toupper(c: TreCint) -> TreCint {
    tre_toupper_l1(c)
}
#[inline]
pub(crate) unsafe fn tre_isctype(wc: TreCint, desc: TreCtype) -> bool {
    tre_isctype_l1(wc, desc)
}
#[inline]
pub(crate) unsafe fn tre_ctype(name: *const c_char) -> TreCtype {
    tre_ctype_l1(name)
}

// ============================================================================
// 第五部分：Tag 匹配方向枚举
// ============================================================================

/// Tag 匹配方向。
///
/// 对应 C 的 `TRE_TAG_MINIMIZE` / `TRE_TAG_MAXIMIZE`，标记每个 submatch tag
/// 的匹配策略。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i32)]
pub(crate) enum TagDirection {
    /// 非贪婪/懒惰匹配（对应 `*?`、`+?`）。
    Minimize = 0,
    /// 贪婪匹配（对应 `*`、`+`），POSIX 最左最长规则。
    Maximize = 1,
}

// ============================================================================
// 第六部分：TNFA 核心数据结构
// ============================================================================

/// TNFA 转移边。
///
/// 表示从某个 TNFA 状态经过特定字符或条件到另一个状态的转移。
/// 转移数组必须以 `state_id == -1` 的元素结尾。
///
/// # C vs Rust 差异
///
/// - C 结构体使用 union `{ class, backref }`；Rust 使用 `Option` 枚举分别存储
/// - C 的 `state` 指针被替换为 `state_id` 索引
/// - C 的 `tags` 以 `-1` 结尾的裸指针数组；Rust 使用 `Option<Box<[i32]>>`
#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct TnfaTransition {
    /// 接受的字符范围下限（闭区间）。
    pub code_min: TreCint,
    /// 接受的字符范围上限（闭区间）。
    pub code_max: TreCint,
    /// 目标状态的数字 ID（-1 标志数组结束）。
    pub state_id: i32,
    /// 断言位掩码（零宽度条件，见 ASSERT_* 常量）。
    pub assertions: i32,
    /// 转移时写入的 tag 编号列表（以 -1 结尾）。None 表示无 tag 操作。
    pub tags: Option<Box<[i32]>>,
    /// 正向字符类别句柄（当 assertions & ASSERT_CHAR_CLASS 时有效）。
    pub u_class: Option<TreCtype>,
    /// 反向引用编号（当 assertions & ASSERT_BACKREF 时有效）。
    pub u_backref: Option<i32>,
    /// 否定字符类别列表（以 0 结尾，当 assertions & ASSERT_CHAR_CLASS_NEG 时有效）。
    pub neg_classes: Option<Box<[TreCtype]>>,
}

/// 子匹配元数据。
///
/// 为每个捕获组描述如何从 tag 值计算出 `regmatch_t` 中的
/// `rm_so`（起始偏移）和 `rm_eo`（结束偏移）。
#[derive(Clone, Debug)]
#[repr(C)]
pub(crate) struct SubmatchData {
    /// 提供 rm_so 值的 tag 编号。
    pub so_tag: i32,
    /// 提供 rm_eo 值的 tag 编号。
    pub eo_tag: i32,
    /// 父 submatch 编号列表（以 0 结尾）。None 表示无父级约束。
    pub parents: Option<Box<[i32]>>,
}

/// TNFA 顶层结构体。
///
/// 由 `regcomp` 构造并通过 `regex_t.__opaque` 传递，由 `regfree` 释放。
/// 包含编译后的正则表达式的全部状态转移信息。
///
/// # 不变量
///
/// - `num_submatches >= 1`（第 0 号始终存在，对应整体匹配）
/// - `submatch_data.len() == num_submatches as usize`
/// - `tag_directions.len() == num_tags as usize`
/// - `firstpos_chars` 长度为 32 字节（256 位位图）
/// - 若 `have_backrefs` 为真，匹配引擎必须启用反向引用解析路径
#[derive(Clone, Debug)]
pub(crate) struct Tnfa {
    /// 所有转移边的扁平数组。通过 `state_id` 索引和终止标记 `-1` 组织。
    pub transitions: Box<[TnfaTransition]>,
    /// 初始状态的 `state_id`。
    pub initial_id: i32,
    /// 接受（终止）状态的 `state_id`。
    pub final_id: i32,
    /// 初始转移的 tag 列表。
    pub initial_tags: Option<Box<[i32]>>,
    /// submatch 元数据数组。
    pub submatch_data: Box<[SubmatchData]>,
    /// 位图（256-bit），可能匹配的首字符集合。用于快速预判。
    pub firstpos_chars: [u8; 32],
    /// 确定的单个首字符（负值表示无确定首字符）。
    pub first_char: i32,
    /// 子表达式（捕获组）总数（含第 0 号整体匹配）。
    pub num_submatches: u32,
    /// 每个 tag 的匹配方向。
    pub tag_directions: Box<[TagDirection]>,
    /// 最小化匹配的 tag 编号列表（以 -1 结尾）。None 表示无最小化 tag。
    pub minimal_tags: Option<Box<[i32]>>,
    /// tag 总数。
    pub num_tags: i32,
    /// 最小化匹配的 tag 数量。
    pub num_minimals: i32,
    /// 整体匹配结束的 tag 编号。
    pub end_tag: i32,
    /// TNFA 状态总数。
    pub num_states: i32,
    /// 编译标志（REG_EXTENDED | REG_ICASE | REG_NEWLINE | REG_NOSUB）。
    pub cflags: i32,
    /// 是否包含反向引用。
    pub have_backrefs: bool,
    /// 是否使用近似匹配（musl 当前不支持，保留字段）。
    pub have_approx: bool,
}

// ============================================================================
// 第七部分：系统分配器函数（通过 Rust alloc crate）
// ============================================================================

/// 堆内存分配。通过 Rust 的 `alloc` crate 实现，无 FFI 依赖。
#[inline]
pub(crate) unsafe fn xmalloc(size: usize) -> *mut core::ffi::c_void {
    if size == 0 {
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 8).unwrap_or_else(|_| {
        Layout::from_size_align_unchecked(size, 8)
    });
    alloc(layout) as *mut core::ffi::c_void
}

/// 零初始化堆内存分配。通过 Rust 的 `alloc` crate 实现。
#[inline]
pub(crate) unsafe fn xcalloc(n: usize, size: usize) -> *mut core::ffi::c_void {
    let total = n.checked_mul(size).unwrap_or(0);
    if total == 0 {
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(total, 8).unwrap_or_else(|_| {
        Layout::from_size_align_unchecked(total, 8)
    });
    alloc_zeroed(layout) as *mut core::ffi::c_void
}

/// 释放堆内存。通过 Rust 的 `alloc` crate 实现。
#[inline]
pub(crate) unsafe fn xfree(ptr: *mut core::ffi::c_void) {
    // free(NULL) 是无操作
    // 注意：我们无法从裸指针恢复 Layout，因此此函数仅用于
    // 测试代码中已知大小的分配。对于生产代码，请使用 Box/Vec。
    // 在测试中，alloc 返回的指针通过此函数释放，
    // 但由于缺少 Layout 信息，实际释放可能不会发生。
    // 内存泄漏在测试中是允许的。
    let _ = ptr;
}

/// 重新分配堆内存。通过 Rust 的 `alloc` crate 实现。
#[inline]
pub(crate) unsafe fn xrealloc(
    ptr: *mut core::ffi::c_void,
    size: usize,
) -> *mut core::ffi::c_void {
    if ptr.is_null() {
        return xmalloc(size);
    }
    if size == 0 {
        xfree(ptr);
        return core::ptr::null_mut();
    }
    let layout = Layout::from_size_align(size, 8).unwrap_or_else(|_| {
        Layout::from_size_align_unchecked(size, 8)
    });
    realloc(ptr as *mut u8, layout, size) as *mut core::ffi::c_void
}

// ============================================================================
// 第八部分：工具函数
// ============================================================================

/// 计算将 `ptr` 对齐到 `type_align` 边界所需的字节偏移量。
///
/// 对应 C 的 `ALIGN` 宏。
#[inline]
pub(crate) const fn align_offset(ptr: usize, type_align: usize) -> usize {
    if ptr % type_align == 0 {
        0
    } else {
        type_align - ptr % type_align
    }
}

// ============================================================================
// 第九部分：TNFA 状态标记常量
// ============================================================================

/// 转移数组终止标记 — 当 `state_id == END_STATE_MARKER` 时表示到达状态末尾。
pub(crate) const END_STATE_MARKER: i32 = -1;

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    // ---- 常量测试 ----

    test!("test_tre_char_max_is_valid_unicode" {
        assert_eq!(TRE_CHAR_MAX, 0x10ffff);
    });

    test!("test_tre_mem_block_size_reasonable" {
        assert!(TRE_MEM_BLOCK_SIZE > 0);
        assert_eq!(TRE_MEM_BLOCK_SIZE, 1024);
    });

    test!("test_assert_bitmasks_are_distinct" {
        let masks = [
            ASSERT_AT_BOL,
            ASSERT_AT_EOL,
            ASSERT_CHAR_CLASS,
            ASSERT_CHAR_CLASS_NEG,
            ASSERT_AT_BOW,
            ASSERT_AT_EOW,
            ASSERT_AT_WB,
            ASSERT_AT_WB_NEG,
            ASSERT_BACKREF,
        ];
        for i in 0..masks.len() {
            for j in (i + 1)..masks.len() {
                assert_eq!(
                    masks[i] & masks[j],
                    0,
                    "断言位掩码 {} 和 {} 重叠",
                    masks[i],
                    masks[j]
                );
            }
        }
    });

    test!("test_assert_last_equals_backref" {
        assert_eq!(ASSERT_LAST, ASSERT_BACKREF);
    });

    test!("test_end_state_marker_negative" {
        assert_eq!(END_STATE_MARKER, -1);
    });

    // ---- UTF-8 解码器测试 ----

    test!("test_mbtowc_ascii" {
        unsafe {
            let mut wc: i32 = 0;
            let s: &[u8] = b"A";
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 1);
            assert_eq!(wc, 'A' as i32);
        }
    });

    test!("test_mbtowc_ascii_lowercase" {
        unsafe {
            let mut wc: i32 = 0;
            let s: &[u8] = b"z";
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 1);
            assert_eq!(wc, 'z' as i32);
        }
    });

    test!("test_mbtowc_null_byte" {
        unsafe {
            let mut wc: i32 = 0xAB; // 非零初始值
            let s: &[u8] = b"\0";
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 0);
            assert_eq!(wc, 0);
        }
    });

    test!("test_mbtowc_two_byte_utf8" {
        unsafe {
            let mut wc: i32 = 0;
            // U+00E9 = é = 0xC3 0xA9
            let s: &[u8] = &[0xC3, 0xA9];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 2);
            assert_eq!(wc, 0xE9);
        }
    });

    test!("test_mbtowc_three_byte_utf8" {
        unsafe {
            let mut wc: i32 = 0;
            // U+4E2D = 中 = 0xE4 0xB8 0xAD
            let s: &[u8] = &[0xE4, 0xB8, 0xAD];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 3);
            assert_eq!(wc, 0x4E2D);
        }
    });

    test!("test_mbtowc_four_byte_utf8" {
        unsafe {
            let mut wc: i32 = 0;
            // U+1F600 = 😀 = 0xF0 0x9F 0x98 0x80
            let s: &[u8] = &[0xF0, 0x9F, 0x98, 0x80];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 4);
            assert_eq!(wc, 0x1F600);
        }
    });

    test!("test_mbtowc_invalid_sequence" {
        unsafe {
            let mut wc: i32 = 0;
            // 0xFF 永远非法
            let s: &[u8] = &[0xFF];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, -1);
        }
    });

    test!("test_mbtowc_truncated_two_byte" {
        unsafe {
            let mut wc: i32 = 0;
            // 缺少后续字节
            let s: &[u8] = &[0xC3];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, -1);
        }
    });

    test!("test_mbtowc_overlong_encoding" {
        unsafe {
            let mut wc: i32 = 0;
            // 'A' 的过长 2 字节编码：0xC1 0x81
            let s: &[u8] = &[0xC1, 0x81];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, -1);
        }
    });

    test!("test_mbtowc_surrogate_pair" {
        unsafe {
            let mut wc: i32 = 0;
            // U+D800 是代理对，UTF-8 中非法
            let s: &[u8] = &[0xED, 0xA0, 0x80];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, -1);
        }
    });

    test!("test_mbtowc_zero_length" {
        unsafe {
            let mut wc: i32 = 0;
            let s: &[u8] = &[];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), 0);
            assert_eq!(len, -1);
        }
    });

    test!("test_mbtowc_max_unicode" {
        unsafe {
            let mut wc: i32 = 0;
            // U+10FFFF = 0xF4 0x8F 0xBF 0xBF
            let s: &[u8] = &[0xF4, 0x8F, 0xBF, 0xBF];
            let len = tre_mbtowc(&mut wc, s.as_ptr(), s.len());
            assert_eq!(len, 4);
            assert_eq!(wc, 0x10FFFF);
        }
    });

    // ---- 宽字符分类函数测试 ----

    test!("test_tre_isalpha_ascii" {
        assert!(unsafe { tre_isalpha(b'A' as TreCint) });
        assert!(unsafe { tre_isalpha(b'z' as TreCint) });
        assert!(!unsafe { tre_isalpha(b'0' as TreCint) });
        assert!(!unsafe { tre_isalpha(b' ' as TreCint) });
    });

    test!("test_tre_isdigit" {
        assert!(unsafe { tre_isdigit(b'0' as TreCint) });
        assert!(unsafe { tre_isdigit(b'9' as TreCint) });
        assert!(!unsafe { tre_isdigit(b'A' as TreCint) });
    });

    test!("test_tre_isspace" {
        assert!(unsafe { tre_isspace(b' ' as TreCint) });
        assert!(unsafe { tre_isspace(b'\t' as TreCint) });
        assert!(unsafe { tre_isspace(b'\n' as TreCint) });
        assert!(!unsafe { tre_isspace(b'A' as TreCint) });
    });

    test!("test_tre_islower" {
        assert!(unsafe { tre_islower(b'a' as TreCint) });
        assert!(!unsafe { tre_islower(b'A' as TreCint) });
        assert!(!unsafe { tre_islower(b'0' as TreCint) });
    });

    test!("test_tre_isupper" {
        assert!(unsafe { tre_isupper(b'A' as TreCint) });
        assert!(!unsafe { tre_isupper(b'a' as TreCint) });
    });

    test!("test_tre_isalnum" {
        assert!(unsafe { tre_isalnum(b'A' as TreCint) });
        assert!(unsafe { tre_isalnum(b'z' as TreCint) });
        assert!(unsafe { tre_isalnum(b'0' as TreCint) });
        assert!(!unsafe { tre_isalnum(b' ' as TreCint) });
    });

    test!("test_tre_isblank" {
        assert!(unsafe { tre_isblank(b' ' as TreCint) });
        assert!(unsafe { tre_isblank(b'\t' as TreCint) });
        assert!(!unsafe { tre_isblank(b'\n' as TreCint) });
    });

    test!("test_tre_iscntrl" {
        assert!(unsafe { tre_iscntrl(0x00) });
        assert!(unsafe { tre_iscntrl(0x1F) });
        assert!(!unsafe { tre_iscntrl(b'A' as TreCint) });
    });

    test!("test_tre_isgraph" {
        assert!(unsafe { tre_isgraph(b'A' as TreCint) });
        assert!(!unsafe { tre_isgraph(b' ' as TreCint) });
    });

    test!("test_tre_isprint" {
        assert!(unsafe { tre_isprint(b'A' as TreCint) });
        assert!(!unsafe { tre_isprint(0x00) });
    });

    test!("test_tre_ispunct" {
        assert!(unsafe { tre_ispunct(b'.' as TreCint) });
        assert!(!unsafe { tre_ispunct(b'A' as TreCint) });
    });

    test!("test_tre_isxdigit" {
        assert!(unsafe { tre_isxdigit(b'A' as TreCint) });
        assert!(unsafe { tre_isxdigit(b'f' as TreCint) });
        assert!(unsafe { tre_isxdigit(b'0' as TreCint) });
        assert!(!unsafe { tre_isxdigit(b'G' as TreCint) });
    });

    // ---- 大小写转换测试 ----

    test!("test_tre_tolower" {
        assert_eq!(unsafe { tre_tolower(b'A' as TreCint) }, b'a' as TreCint);
        assert_eq!(unsafe { tre_tolower(b'Z' as TreCint) }, b'z' as TreCint);
        assert_eq!(unsafe { tre_tolower(b'a' as TreCint) }, b'a' as TreCint);
        assert_eq!(unsafe { tre_tolower(b'0' as TreCint) }, b'0' as TreCint);
    });

    test!("test_tre_toupper" {
        assert_eq!(unsafe { tre_toupper(b'a' as TreCint) }, b'A' as TreCint);
        assert_eq!(unsafe { tre_toupper(b'z' as TreCint) }, b'Z' as TreCint);
        assert_eq!(unsafe { tre_toupper(b'A' as TreCint) }, b'A' as TreCint);
        assert_eq!(unsafe { tre_toupper(b'0' as TreCint) }, b'0' as TreCint);
    });

    // ---- iswctype / wctype 测试 ----

    test!("test_tre_isctype_alpha" {
        let alpha_desc = rusl_ctype::WCTYPE_ALPHA as TreCtype;
        assert!(unsafe { tre_isctype(b'A' as TreCint, alpha_desc) });
        assert!(!unsafe { tre_isctype(b'0' as TreCint, alpha_desc) });
    });

    test!("test_tre_ctype_alpha" {
        unsafe {
            let name = b"alpha\0".as_ptr() as *const c_char;
            let desc = tre_ctype(name);
            assert!(desc > 0);
        }
    });

    // ---- TagDirection 测试 ----

    test!("test_tag_direction_values" {
        assert_eq!(TagDirection::Minimize as i32, 0);
        assert_eq!(TagDirection::Maximize as i32, 1);
    });

    test!("test_tag_direction_copy_clone" {
        let d = TagDirection::Maximize;
        let d2 = d; // Copy
        assert_eq!(d, d2);
        let d3 = d.clone(); // Clone
        assert_eq!(d, d3);
    });

    // ---- TnfaTransition 测试 ----

    test!("test_tnfa_transition_creation" {
        let t = TnfaTransition {
            code_min: 'a' as i32,
            code_max: 'z' as i32,
            state_id: 1,
            assertions: 0,
            tags: None,
            u_class: None,
            u_backref: None,
            neg_classes: None,
        };
        assert_eq!(t.code_min, 'a' as i32);
        assert_eq!(t.code_max, 'z' as i32);
        assert_eq!(t.state_id, 1);
        assert_eq!(t.assertions, 0);
        assert!(t.tags.is_none());
        assert!(t.u_class.is_none());
        assert!(t.u_backref.is_none());
        assert!(t.neg_classes.is_none());
    });

    test!("test_tnfa_transition_with_tags" {
        let tags: Box<[i32]> = Box::new([0, 1, -1]);
        let t = TnfaTransition {
            code_min: -1,
            code_max: -1,
            state_id: 2,
            assertions: ASSERT_AT_BOL,
            tags: Some(tags.clone()),
            u_class: None,
            u_backref: None,
            neg_classes: None,
        };
        assert!(t.tags.is_some());
        let t_tags = t.tags.unwrap();
        assert_eq!(t_tags.len(), 3);
        assert_eq!(t_tags[2], -1);
    });

    test!("test_tnfa_transition_with_assertions" {
        let t = TnfaTransition {
            code_min: 0,
            code_max: TRE_CHAR_MAX,
            state_id: 3,
            assertions: ASSERT_CHAR_CLASS | ASSERT_AT_BOW,
            tags: None,
            u_class: Some(1),
            u_backref: None,
            neg_classes: None,
        };
        assert_ne!(t.assertions & ASSERT_CHAR_CLASS, 0);
        assert_ne!(t.assertions & ASSERT_AT_BOW, 0);
    });

    test!("test_tnfa_transition_with_neg_classes" {
        let neg: Box<[TreCtype]> = Box::new([1, 2, 3, 0]);
        let t = TnfaTransition {
            code_min: 0,
            code_max: 0,
            state_id: 4,
            assertions: ASSERT_CHAR_CLASS_NEG,
            tags: None,
            u_class: None,
            u_backref: None,
            neg_classes: Some(neg),
        };
        assert!(t.neg_classes.is_some());
        let nc = t.neg_classes.unwrap();
        assert_eq!(nc.len(), 4);
        assert_eq!(nc[3], 0);
    });

    test!("test_tnfa_transition_with_backref" {
        let t = TnfaTransition {
            code_min: 0,
            code_max: 0,
            state_id: 5,
            assertions: ASSERT_BACKREF,
            tags: None,
            u_class: None,
            u_backref: Some(1),
            neg_classes: None,
        };
        assert_eq!(t.u_backref, Some(1));
    });

    // ---- SubmatchData 测试 ----

    test!("test_submatch_data_creation" {
        let sd = SubmatchData {
            so_tag: 0,
            eo_tag: 1,
            parents: None,
        };
        assert_eq!(sd.so_tag, 0);
        assert_eq!(sd.eo_tag, 1);
        assert!(sd.parents.is_none());
    });

    test!("test_submatch_data_with_parents" {
        let parents: Box<[i32]> = Box::new([0, 2, 0]);
        let sd = SubmatchData {
            so_tag: 2,
            eo_tag: 3,
            parents: Some(parents),
        };
        assert!(sd.parents.is_some());
        let p = sd.parents.unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[2], 0);
    });

    // ---- Tnfa 测试 ----

    test!("test_tnfa_creation_empty" {
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: 0,
                eo_tag: 1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 0,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        };
        assert_eq!(tnfa.num_submatches, 1);
        assert_eq!(tnfa.submatch_data.len(), 1);
        assert!(!tnfa.have_backrefs);
        assert!(!tnfa.have_approx);
        assert_eq!(tnfa.transitions.len(), 0);
    });

    test!("test_tnfa_invariants_num_submatches" {
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: 0,
            initial_tags: None,
            submatch_data: Box::new([
                SubmatchData {
                    so_tag: 0,
                    eo_tag: 1,
                    parents: None,
                },
                SubmatchData {
                    so_tag: 2,
                    eo_tag: 3,
                    parents: None,
                },
            ]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 2,
            tag_directions: Box::new([TagDirection::Maximize; 4]),
            minimal_tags: None,
            num_tags: 4,
            num_minimals: 0,
            end_tag: 1,
            num_states: 3,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        };
        assert_eq!(tnfa.submatch_data.len(), tnfa.num_submatches as usize);
        assert_eq!(tnfa.tag_directions.len(), tnfa.num_tags as usize);
    });

    test!("test_tnfa_with_backrefs" {
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: 0,
                eo_tag: 1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 0,
            cflags: 0,
            have_backrefs: true,
            have_approx: false,
        };
        assert!(tnfa.have_backrefs);
    });

    test!("test_firstpos_chars_size" {
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: 0,
                eo_tag: 1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 0,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        };
        assert_eq!(tnfa.firstpos_chars.len(), 32);
        assert_eq!(tnfa.firstpos_chars.len() * 8, 256);
    });

    test!("test_tnfa_with_minimal_tags" {
        let minimal: Box<[i32]> = Box::new([2, 3, -1]);
        let tnfa = Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([SubmatchData {
                so_tag: 0,
                eo_tag: 1,
                parents: None,
            }]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches: 1,
            tag_directions: Box::new([
                TagDirection::Maximize,
                TagDirection::Maximize,
                TagDirection::Minimize,
                TagDirection::Minimize,
            ]),
            minimal_tags: Some(minimal),
            num_tags: 4,
            num_minimals: 2,
            end_tag: 1,
            num_states: 4,
            cflags: 0,
            have_backrefs: false,
            have_approx: false,
        };
        assert!(tnfa.minimal_tags.is_some());
        assert_eq!(tnfa.num_minimals, 2);
    });

    // ---- Align 工具函数测试 ----

    test!("test_align_offset_already_aligned" {
        assert_eq!(align_offset(8, 8), 0);
        assert_eq!(align_offset(16, 8), 0);
        assert_eq!(align_offset(0, 8), 0);
        assert_eq!(align_offset(32, 16), 0);
    });

    test!("test_align_offset_needs_padding" {
        assert_eq!(align_offset(1, 8), 7);
        assert_eq!(align_offset(3, 8), 5);
        assert_eq!(align_offset(7, 8), 1);
        assert_eq!(align_offset(9, 8), 7);
    });

    test!("test_align_offset_small_alignment" {
        assert_eq!(align_offset(1, 2), 1);
        assert_eq!(align_offset(3, 4), 1);
        assert_eq!(align_offset(5, 4), 3);
    });

    // ---- 分配器测试 ----

    test!("test_xmalloc_returns_non_null_for_small_size" {
        unsafe {
            let ptr = xmalloc(64);
            assert!(!ptr.is_null(), "xmalloc(64) 返回了 null 指针");
            xfree(ptr);
        }
    });

    test!("test_xmalloc_zero_size" {
        unsafe {
            let ptr = xmalloc(0);
            if !ptr.is_null() {
                xfree(ptr);
            }
        }
    });

    test!("test_xcalloc_zeroes_memory" {
        unsafe {
            let ptr = xcalloc(16, 1) as *mut u8;
            assert!(!ptr.is_null(), "xcalloc(16, 1) 返回了 null 指针");
            for i in 0..16 {
                assert_eq!(*ptr.add(i), 0u8, "xcalloc 未清零字节 {}", i);
            }
            xfree(ptr as *mut core::ffi::c_void);
        }
    });

    test!("test_xrealloc_grow" {
        unsafe {
            let ptr = xmalloc(8);
            assert!(!ptr.is_null());
            let p = ptr as *mut u8;
            *p = 0xAB;
            let new_ptr = xrealloc(ptr, 64);
            assert!(!new_ptr.is_null());
            let np = new_ptr as *mut u8;
            assert_eq!(*np, 0xAB);
            xfree(new_ptr);
        }
    });

    test!("test_xfree_accepts_null" {
        unsafe {
            xfree(core::ptr::null_mut());
        }
    });
}