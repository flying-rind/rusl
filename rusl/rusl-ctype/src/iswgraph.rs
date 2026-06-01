//! iswgraph — 宽字符图形字符判断。
//! 对应 musl src/ctype/iswgraph.c
//!
//! 图形字符 = 可打印字符 - 空白字符。
//! 等价于 `iswprint(wc) != 0 && iswspace(wc) == 0`。

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 判断宽字符是否为图形字符 (可打印且非空白)。
///
/// 图形字符定义: `iswprint(wc) != 0 && iswspace(wc) == 0`
///
/// 实现采用短路求值策略: 先检查 `iswspace` (通常更便宜),
/// 若为空白字符则直接返回 0, 否则再调用 `iswprint`。
///
/// # 安全性
///
/// 此函数标记为 `unsafe` 以保持 C ABI 签名兼容。
/// 实际调用无内存安全性风险。
#[no_mangle]
pub unsafe extern "C" fn iswgraph(wc: wint_t) -> c_int {
    __iswgraph_l(wc, core::ptr::null_mut())
}

/// locale 感知的图形字符判断。
///
/// 等价于 `!iswspace_l(wc, l) && iswprint_l(wc, l)`。
///
/// # 安全性
///
/// - `l`: 必须为有效的 locale 句柄, 或 `NULL` 表示 C locale。
#[no_mangle]
pub unsafe extern "C" fn iswgraph_l(wc: wint_t, l: locale_t) -> c_int {
    __iswgraph_l(wc, l)
}

/// 内部实现函数。供 [`iswctype_l`] 等内部分类函数调用。
///
/// [`iswgraph`] 等价于 `__iswgraph_l(wc, core::ptr::null_mut())`。
/// 内部调用 [`__iswspace_l`] 和 [`__iswprint_l`] 进行短路求值:
/// 先检查是否为空白字符 (更便宜的检查), 若是则直接返回 0,
/// 否则委托给 [`__iswprint_l`] 进行可打印字符判断。
///
/// # 安全性
///
/// 同 [`iswgraph_l`]。
pub(crate) fn __iswgraph_l(wc: wint_t, l: locale_t) -> c_int {
    // 短路求值: 先检查 iswspace (通常为 O(1) 区间检查)
    // 若为空白字符则直接返回 0, 避免不必要的 iswprint 调用
    if super::iswspace::__iswspace_l(wc) {
        return 0;
    }
    super::iswprint::__iswprint_l(wc, l)
}
