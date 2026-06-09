//! iswupper — 测试宽字符是否为大写字母。
//! 对应 musl src/ctype/iswupper.c
//!
//! 通过检测 `towlower(wc) != wc` 判断宽字符是否为大写字母。
//! 与 iswlower 对称，利用大小写转换表反向推断：若字符可被转为小写
//! 且映射结果不等于自身，则原字符为大写字母。

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 测试宽字符 wc 是否为大写字母。
///
/// 通过调用 `towlower(wc)` 并检查 `towlower(wc) != wc` 来判断。
/// 若字符存在对应的小写形式且转换结果不等于自身，则为大写字母。
///
/// # 参数
///
/// * `wc` - 宽字符值，类型为 `wint_t` (`c_uint`)，任意宽字符值（含 `WEOF`）
///
/// # 返回
///
/// * 如果 wc 是大写字母（`towlower(wc) != wc`），返回非零值
/// * 如果 wc 不是大写字母或 `wc == WEOF`，返回 0
///
/// # Safety
///
#[no_mangle]
pub extern "C" fn iswupper(wc: wint_t) -> c_int {
    (super::towlower(wc) != wc) as c_int
}

/// 测试宽字符 c 是否为大写字母（带 locale 参数）。
///
/// `locale` 参数传递给 `towlower` 以支持 locale 感知的大小写转换。
/// 当前 musl 实现中多为 C/POSIX locale。
///
/// # 参数
///
/// * `c` - 宽字符值
/// * `l` - locale 句柄，传递给内部的 `towlower` 查找
///
#[no_mangle]
pub extern "C" fn iswupper_l(c: wint_t, _l: locale_t) -> c_int {
    (super::towlower(c) != c) as c_int
}

/// iswupper 和 iswupper_l 的内部委托实现。
///
/// 核心逻辑: `towlower(wc, locale) != wc`。
/// 依赖 `towlower` 的 Unicode 大小写映射表执行 O(1) 查找。
///
/// `iswupper(wc)` 等价于 `__iswupper_l(wc, core::ptr::null_mut())`
/// `iswupper_l(c, l)` 等价于 `__iswupper_l(c, l)`
pub(crate) fn __iswupper_l(c: wint_t, _l: locale_t) -> c_int {
    (super::towlower(c) != c) as c_int
}
