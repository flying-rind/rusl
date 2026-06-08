//! ispunct —— 判断字符是否为标点符号。
//! 对应 musl src/ctype/ispunct.c
//!
//! 标点符号定义为可打印图形字符中排除字母和数字的部分，
//! 即 `isgraph(c) && !isalnum(c)` 为 true 的字符。
//!
//! musl 通过组合 `isgraph` 和 `isalnum` 实现，避免独立维护标点符号位图表。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_uint};
use crate::import::__locale_struct;

/// C 标准 ispunct —— 判断字符是否为标点符号。
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
/// * 若 `c` 是标点符号（可打印图形字符但非字母非数字），返回非零值（musl 中为 1）。
/// * 其他字符或 `EOF` 返回 0。
///
/// # 行为说明
///
/// 标点符号 = `isgraph(c) && !isalnum(c)`。
/// 纯函数，无副作用，无内部可变状态，完全线程安全。
///
/// # 算法
///
/// 组合 isgraph 和 isalnum 判定：
/// - isgraph: `(unsigned)c - 0x21 < 0x5e`（检查 0x21-0x7E）
/// - isalnum: `isalpha(c) || isdigit(c)`
///   - isalpha: `((unsigned)c|32)-'a' < 26`
///   - isdigit: `(unsigned)c-'0' < 10`
///
/// 对应 musl `src/ctype/ispunct.c`: `return isgraph(c) && !isalnum(c);`
#[no_mangle]
pub extern "C" fn ispunct(c: c_int) -> c_int {
    let cu = c as c_uint;
    // isgraph: 0x21 <= cu <= 0x7E
    let is_graph = cu.wrapping_sub(0x21) < 0x5e;
    // isalnum: isalpha || isdigit
    // isalpha: (cu|32) in ['a'..='z']
    // isdigit: cu in ['0'..='9']
    let is_alnum = (cu | 32).wrapping_sub(b'a' as c_uint) < 26
        || cu.wrapping_sub(b'0' as c_uint) < 10;
    (is_graph && !is_alnum) as c_int
}

/// ispunct_l_impl —— ispunct_l 的内部实现体（C 中对应 `__ispunct_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，本函数作为 `ispunct_l` 的委托目标。
/// musl 中 `ispunct_l` 通过 `weak_alias(__ispunct_l, ispunct_l)` 链接至此。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `ispunct`。
/// * `l` - 类型为 `*mut __locale_struct`，指向 locale 结构的指针。
///   在 musl 中**被忽略**，所有 locale 下行为一致。
///
/// # 返回值
///
/// 完全等效于 `ispunct(c)` 的返回值。
#[inline]
fn ispunct_l_impl(c: c_int, _l: *mut __locale_struct) -> c_int {
    ispunct(c)
}

/// ispunct_l —— locale-aware 标点符号判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `ispunct` 完全一致。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `ispunct`。
/// * `l` - 类型为 `*mut __locale_struct`，在 musl 中被忽略。
///   调用者可以传递 `null_mut()`。
///
/// # 返回值
///
/// 完全等效于 `ispunct(c)` 的返回值。
#[no_mangle]
pub extern "C" fn ispunct_l(c: c_int, l: *mut __locale_struct) -> c_int {
    ispunct_l_impl(c, l)
}