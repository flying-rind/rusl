//! isspace —— 判断字符是否为空白字符。
//! 对应 musl src/ctype/isspace.c
//!
//! 使用紧凑的无符号区间技巧：
//! `(c as c_uint).wrapping_sub('\t' as c_uint) < 5` 覆盖 `'\t'`(9) 到 `'\r'`(13)
//! 五个连续空白字符，再单独检查空格 `' '`(32)。
//! 共 1 次无符号比较 + 1 次相等比较，无分支预测开销。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_uint};
use rusl_internal::libc::__locale_struct;

/// C 标准 isspace —— 判断字符是否为空白字符。
///
/// [Visibility]: External —— POSIX.1-2001 标准函数，`<ctype.h>` 声明，ABI 兼容。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`（对应 C 的 `int`），值必须可表示为 `unsigned char`
///   或等于 `EOF` (-1)。
///
/// # 返回值
///
/// * 若 `c` 是 C 标准空白字符（`' '`、`'\t'`、`'\n'`、`'\v'`、`'\f'`、`'\r'`），
///   返回非零值（musl 中为 1）。
/// * 其他字符或 `EOF` 返回 0。
///
/// # 行为说明
///
/// 纯函数，无副作用，无内部可变状态，完全线程安全。
/// 不依赖 locale 设置。
///
/// # 算法
///
/// 使用无符号区间技巧：
/// - `c == ' '` 检查空格 (0x20)
/// - `(c as c_uint).wrapping_sub('\t' as c_uint) < 5` 覆盖
///   `'\t'`(9) 到 `'\r'`(13) 五个连续空白字符
///
/// 对应 musl `src/ctype/isspace.c`: `return c == ' ' || (unsigned)c-'\t' < 5;`
#[no_mangle]
pub extern "C" fn isspace(c: c_int) -> c_int {
    (c == b' ' as c_int || (c as c_uint).wrapping_sub(b'\t' as c_uint) < 5) as c_int
}

/// isspace_l_impl —— isspace_l 的内部实现体（C 中对应 `__isspace_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，本函数作为 `isspace_l` 的委托目标。
/// musl 中 `isspace_l` 通过 `weak_alias(__isspace_l, isspace_l)` 链接至此。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `isspace`。
/// * `l` - 类型为 `*mut __locale_struct`，指向 locale 结构的指针。
///   在 musl 中**被忽略**，所有 locale 下行为一致。
///
/// # 返回值
///
/// 完全等效于 `isspace(c)` 的返回值。
#[inline]
fn isspace_l_impl(c: c_int, _l: *mut __locale_struct) -> c_int {
    isspace(c)
}

/// isspace_l —— locale-aware 空白字符判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `isspace` 完全一致。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `isspace`。
/// * `l` - 类型为 `*mut __locale_struct`，在 musl 中被忽略。
///   调用者可以传递 `null_mut()`。
///
/// # 返回值
///
/// 完全等效于 `isspace(c)` 的返回值。
#[no_mangle]
pub extern "C" fn isspace_l(c: c_int, l: *mut __locale_struct) -> c_int {
    isspace_l_impl(c, l)
}