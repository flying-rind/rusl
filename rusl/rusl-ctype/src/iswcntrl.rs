//! iswcntrl — 宽字符控制字符判断。
//! 对应 musl src/ctype/iswcntrl.c

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 判断宽字符是否为控制字符。
///
/// 控制字符覆盖以下 Unicode 码点范围:
/// - C0 控制字符: `wc < 32` (U+0000..U+001F)
/// - DEL + C1 控制字符: `[0x7F, 0x9F]`
/// - 行/段分隔符: `[0x2028, 0x2029]`
/// - 行间注释锚点: `[0xFFF9, 0xFFFB]`
///
/// 若 `wc` 属于任一控制字符范围, 返回非零值; 否则返回 0。
///
/// # 安全性
///
/// 实际调用无内存安全性风险, 因为参数为值类型 `wint_t`。
#[no_mangle]
pub extern "C" fn iswcntrl(wc: wint_t) -> c_int {
    __iswcntrl_l(wc, core::ptr::null_mut())
}

/// locale 感知的控制字符判断。
///
/// 在 C locale 下行为与 [`iswcntrl`] 完全等价。
/// 在其他 locale 下由 LC_CTYPE 类别决定字符分类。
///
#[no_mangle]
pub extern "C" fn iswcntrl_l(wc: wint_t, l: locale_t) -> c_int {
    __iswcntrl_l(wc, l)
}

/// 内部实现函数。供 [`iswctype_l`] 等内部分类函数调用。
///
/// [`iswcntrl`] 等价于 `__iswcntrl_l(wc, core::ptr::null_mut())`。
///
/// 控制字符覆盖以下 Unicode 码点范围:
/// - C0 控制字符: `wc < 32` (U+0000..U+001F)
/// - DEL + C1 控制字符: `wc.wrapping_sub(0x7f) < 33` (U+007F..U+009F)
/// - 行/段分隔符: `wc.wrapping_sub(0x2028) < 2` (U+2028..U+2029)
/// - 行间注释锚点: `wc.wrapping_sub(0xfff9) < 3` (U+FFF9..U+FFFB)
///
/// 使用四个无符号区间检查, O(1) 时间复杂度。
/// `_l` 参数当前保留为 API 兼容占位, 内部实现仅依赖 C locale 行为。
///
/// # 安全性
///
/// 同 [`iswcntrl_l`]。
pub(crate) fn __iswcntrl_l(wc: wint_t, _l: locale_t) -> c_int {
    let w = wc as u32;
    // 四个无符号区间判断, 利用 wrapping_sub 保持与 C 无符号回绕语义一致
    (w < 32
        || w.wrapping_sub(0x7f) < 33
        || w.wrapping_sub(0x2028) < 2
        || w.wrapping_sub(0xfff9) < 3) as c_int
}
