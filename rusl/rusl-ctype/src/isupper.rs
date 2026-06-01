//! isupper —— 判断字符是否为大写字母。
//! 对应 musl src/ctype/isupper.c
//!
//! 使用无符号区间技巧实现：`(c as c_uint).wrapping_sub('A' as c_uint) < 26`
//! 将 26 个大写字母的检查压缩为单次无符号比较，无分支预测开销。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_int, c_uint};
use rusl_internal::libc::__locale_struct;

/// C 标准 isupper —— 判断字符是否为大写字母。
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
/// * 若 `c` 是大写字母（`'A'` 到 `'Z'`，即 0x41 到 0x5A），返回非零值（musl 中为 1）。
/// * 其他字符或 `EOF` 返回 0。
///
/// # 行为说明
///
/// 纯函数，无副作用，无内部可变状态，完全线程安全。
/// 不依赖 locale 设置。
///
/// # 算法
///
/// 使用无符号区间技巧：`(c as c_uint).wrapping_sub('A' as c_uint) < 26`
/// 将 26 个大写字母映射到 [0, 25]，单次比较即可判定。
///
/// 对应 musl `src/ctype/isupper.c`: `return (unsigned)c-'A' < 26;`
#[no_mangle]
pub extern "C" fn isupper(c: c_int) -> c_int {
    ((c as c_uint).wrapping_sub(b'A' as c_uint) < 26) as c_int
}

/// isupper_l_impl —— isupper_l 的内部实现体（C 中对应 `__isupper_l`）。
///
/// [Visibility]: Internal —— musl 内部符号，不直接对外导出。
///
/// Rust 无 `weak_alias` 机制，本函数作为 `isupper_l` 的委托目标。
/// musl 中 `isupper_l` 通过 `weak_alias(__isupper_l, isupper_l)` 链接至此。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `isupper`。
/// * `l` - 类型为 `*mut __locale_struct`，指向 locale 结构的指针。
///   在 musl 中**被忽略**，所有 locale 下行为一致。
///
/// # 返回值
///
/// 完全等效于 `isupper(c)` 的返回值。
#[inline]
fn isupper_l_impl(c: c_int, _l: *mut __locale_struct) -> c_int {
    isupper(c)
}

/// isupper_l —— locale-aware 大写字母判断。
///
/// [Visibility]: External —— POSIX.1-2008 标准函数，`extern "C"` 导出，ABI 兼容。
///
/// 注意：musl 不区分 locale，`l` 参数被忽略，行为与 `isupper` 完全一致。
///
/// # 参数
///
/// * `c` - 类型为 `c_int`，同 `isupper`。
/// * `l` - 类型为 `*mut __locale_struct`，在 musl 中被忽略。
///   调用者可以传递 `null_mut()`。
///
/// # 返回值
///
/// 完全等效于 `isupper(c)` 的返回值。
#[no_mangle]
pub extern "C" fn isupper_l(c: c_int, l: *mut __locale_struct) -> c_int {
    isupper_l_impl(c, l)
}