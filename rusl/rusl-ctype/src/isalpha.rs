//! isalpha — 测试字符是否为英文字母。
//! 对应 musl src/ctype/isalpha.c
//!
//! 本文件包含三个函数:
//! - `isalpha(c)`: 测试字符 c 是否为字母（'a'-'z' 或 'A'-'Z'）。
//! - `__isalpha_l(c, l)`: locale 感知版本，当前实现忽略 locale 参数。
//! - `isalpha_l(c, l)`: `__isalpha_l` 的弱别名，供 POSIX 程序使用。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_void};
use rusl_core::c_types::locale_t;

/// 内部共享实现：无分支位运算判断是否为英文字母。
///
/// 算法核心: `((c_uchar)c|32)-'a' < 26`
/// - 将 c 转为 u32（处理负数/EOF：转为大正数，自然超出范围）
/// - `|32` 将大写字母转为小写（小写字母不变）
/// - 判断结果是否在 'a'..='z' 范围内（差值 < 26）
#[inline]
unsafe fn isalpha_impl(c: c_int) -> c_int {
    // 使用 u32 算术：c as u32 将 EOF(-1) 等负数转为大正数，
    // 自然不满足小于 26 的条件，无需特殊分支处理。
    if ((c as u32 | 32).wrapping_sub(b'a' as u32)) < 26 {
        1
    } else {
        0
    }
}

/// 测试字符 c 是否为英文字母（'a'-'z' 或 'A'-'Z'）。
///
/// # 参数
///
/// - `c`: 待测试的字符，类型为 `c_int`。值必须可表示为 `c_uchar` 或等于 `EOF`（通常为 -1）。
///
/// # 返回值
///
/// - 若 c 是英文字母（大小写均可）：返回非零值（表示真）。
/// - 若 c 不是字母，或 c 为 `EOF`：返回 0（表示假）。
///
/// # Safety
///
/// 标记为 `unsafe` 以保持与 C ABI 兼容。调用者需确保 `c` 可表示为 `c_uchar` 或为 `EOF`。
///
/// # 算法
///
/// 使用无分支位运算 `((c_uchar)c|32)-'a' < 26`：
/// - 通过 `|32` 将大写字母转为小写
/// - 然后判断是否在 'a' 到 'z' 范围内（差小于 26）
/// 时间复杂度 O(1)，无条件分支。
///
/// # 不变量
///
/// 纯函数。无内部可变状态。此 musl 实现不依赖 locale 参数。
#[no_mangle]
pub unsafe extern "C" fn isalpha(c: c_int) -> c_int {
    isalpha_impl(c)
}

/// 测试字符 c 是否为英文字母（locale 感知版本）。
///
/// 此函数接受 locale 参数，但在当前 musl 实现中，
/// locale 参数被忽略，始终使用 C locale 规则。行为与 `isalpha(c)` 一致。
///
/// # 参数
///
/// - `c`: 待测试的字符，类型为 `c_int`。值必须可表示为 `c_uchar` 或等于 `EOF`。
/// - `l`: locale 句柄。可为 `NULL`（表示 C locale）。
///
/// # 返回值
///
/// - 若 c 是英文字母：返回非零值。
/// - 否则：返回 0。
///
/// # Safety
///
/// 标记为 `unsafe` 以保持与 C ABI 兼容。
#[no_mangle]
pub unsafe extern "C" fn __isalpha_l(c: c_int, _l: locale_t) -> c_int {
    // musl 实现忽略 locale 参数，始终使用 C locale 规则
    isalpha_impl(c)
}

/// `__isalpha_l` 的弱别名，供 POSIX 程序使用。
///
/// 此函数通过弱别名机制与 `__isalpha_l` 共享同一实现体。
/// 在 Rust 中无法直接表示弱别名，因此实现中 `isalpha_l` 内部委托给 `__isalpha_l`。
///
/// # 参数
///
/// - `c`: 待测试的字符，类型为 `c_int`。值必须可表示为 `c_uchar` 或等于 `EOF`。
/// - `l`: locale 句柄。必须是非 `NULL` 的有效 `locale_t` 句柄，或为 `LC_GLOBAL_LOCALE` 特殊值。
///
/// # 返回值
///
/// 与 `isalpha(c)` 相同。
///
/// # Safety
///
/// 标记为 `unsafe` 以保持与 C ABI 兼容。
///
/// # 实现说明
///
/// `isalpha_l` 是 `__isalpha_l` 的弱别名（`weak_alias(__isalpha_l, isalpha_l)`）。
/// Rust 不支持弱别名，因此两个函数体都使用 `todo!()`，
/// 实现时 `isalpha_l` 将直接调用 `__isalpha_l` 的逻辑。
#[no_mangle]
pub unsafe extern "C" fn isalpha_l(c: c_int, _l: locale_t) -> c_int {
    // isalpha_l 是 __isalpha_l 的弱别名，共享同一实现体
    isalpha_impl(c)
}
