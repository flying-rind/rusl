//! iswalpha —— 判断宽字符是否为 Unicode 字母。
//! 对应 musl src/ctype/iswalpha.c
//!
//! 使用二级位图查找表覆盖 BMP 及 Supplementary Multilingual Plane
//!（到 U+1FFFF）的所有码点，对 CJK Extension B 范围（U+20000-U+2FFFD）
//! 使用硬编码返回 `true`。
//!
//! 位图表数据来自 Unicode 字符数据库编译生成的 `alpha.h` 头文件。
//! Rust 实现通过 `include_bytes!` 嵌入预处理的二进制位图 `alpha.bin`。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_int;
use rusl_core::c_types::{wint_t, locale_t, WEOF};
use crate::import::__locale_struct;

// ---------------------------------------------------------------------------
// 内部静态数据（编译时常量化）
// ---------------------------------------------------------------------------

/// 静态位图查找表，由 `alpha.h` 编译时生成。
///
/// [Visibility]: Internal —— 模块私有静态只读数据，不对外导出。
/// 对应 C 中 `static const unsigned char table[] = { #include "alpha.h" };`
///
/// 表格结构：
/// - 前 512 字节：一级索引表，每个条目为二级块编号（16..121）
/// - 后续字节：二级位图块，每个块 32 字节，共 106 个唯一块
///
/// 查找算法：
/// - `table[wc>>8]` 获取一级条目（块编号）
/// - `table[block*32 + ((wc&255)>>3)]` 获取二级位图字节
/// - `>>(wc&7) & 1` 提取目标位
///
/// 兼容 `#![no_std]` 环境，编译期嵌入只读段 (.rodata)。
static TABLE: &[u8] = include_bytes!("alpha.bin");

// ---------------------------------------------------------------------------
// 对外导出接口
// ---------------------------------------------------------------------------

/// C 标准 iswalpha —— 判断宽字符是否为 Unicode 字母。
///
/// [Visibility]: External —— POSIX.1-2001 标准函数，`<wctype.h>` 声明，ABI 兼容。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`（即 `c_uint`），任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。
///
/// # 返回值
///
/// * Case 1: `wc` 是 Unicode 字母字符 —— 返回 1。
///   - `wc < 0x20000_u32` 且二级位图查找命中。
///   - `wc` 在 `[0x20000_u32, 0x2fffe_u32)` 范围内（CJK Extension B 区段）。
/// * Case 2: `wc` 不是字母字符 —— 返回 0。
///   - `wc >= 0x2fffe_u32`。
///   - `wc < 0x20000_u32` 但位图查找未命中。
/// * Case 3: `wc == WEOF` (即 `0xffffffff_u32`) —— 返回 0。
///
/// # 行为说明
///
/// 纯函数，无副作用，无内部状态，完全线程安全。
/// 不依赖 locale 设置。
///
/// # Safety
///
/// TABLE 的布局保证了所有合法索引均在数组范围内：
/// - `wc < 0x20000` 时 `wc>>8 < 512`，一级查表始终在界内
/// - 一级条目值范围 [16, 121]，二级索引 `block*32 + offset` 最大 3903 < 3904
/// 因此使用 `unsafe` 的 `get_unchecked` 是安全的且可与 C 的零开销访问等价。
///
/// # 算法
///
/// 对应 musl `src/ctype/iswalpha.c`:
/// ```c
/// if (wc<0x20000U)
///     return (table[table[wc>>8]*32+((wc&255)>>3)]>>(wc&7))&1;
/// if (wc<0x2fffeU)
///     return 1;
/// return 0;
/// ```
#[no_mangle]
pub extern "C" fn iswalpha(wc: wint_t) -> c_int {
    // Phase 1: BMP 及 SMP 到 U+1FFFF —— 二级位图查找
    if wc < 0x20000 {
        let l1_idx = (wc >> 8) as usize;
        // Safety: wc < 0x20000 => wc>>8 < 512 => l1_idx in [0, 511]
        // TABLE.len() = 3904 > 512, so l1_idx is always in bounds
        let l2_block = unsafe { *TABLE.get_unchecked(l1_idx) } as usize;
        let l2_idx = l2_block * 32 + ((wc as usize & 255) >> 3);
        // Safety: l2_block in [16, 121], so l2_idx in [512, 3903]
        // TABLE.len() = 3904, so l2_idx is always in bounds
        let byte = unsafe { *TABLE.get_unchecked(l2_idx) };
        return ((byte >> (wc & 7)) & 1) as c_int;
    }
    // Phase 2: CJK Extension B —— U+20000 到 U+2FFFD（含）
    if wc < 0x2FFFE {
        return 1;
    }
    // Phase 3: 越界（包括 WEOF = 0xFFFFFFFF）
    0
}

/// iswalpha_l_impl —— iswalpha_l 的内部实现体（C 中对应 `__iswalpha_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，更名为 `iswalpha_l_impl` 并作为模块内部函数。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
/// * `l` - 类型为 `locale_t`（`*mut __locale_struct`），指向有效的 locale 结构
///   或为 `null_mut()`（表示 C locale）。在 musl 中**被忽略**。
///
/// # 返回值
///
/// 完全等效于 `iswalpha(wc)` 的返回值。
#[inline]
pub(crate) fn iswalpha_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswalpha(wc)
}

/// iswalpha_l —— locale-aware 宽字符字母判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `iswalpha` 完全一致。
/// Rust 中 `iswalpha_l` 为独立的 `extern "C"` 函数，内部委托给 `iswalpha_l_impl`
///（相当于 C 中 `weak_alias` 的效果）。
///
/// # 参数
///
/// * `wc` - 类型为 `wint_t`，同 `iswalpha`。
/// * `l` - 类型为 `locale_t`，在 musl 中被忽略。
///
/// # 返回值
///
/// 完全等效于 `iswalpha(wc)` 的返回值。
#[no_mangle]
pub extern "C" fn iswalpha_l(wc: wint_t, l: locale_t) -> c_int {
    iswalpha_l_impl(wc, l)
}