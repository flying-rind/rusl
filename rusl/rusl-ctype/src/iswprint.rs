//! iswprint — 宽字符可打印字符判断。
//! 对应 musl src/ctype/iswprint.c
//!
//! 采用五阶段决策树算法, O(1) 时间复杂度, 针对 ASCII 可打印字符热路径
//! 进行了位运算优化。

use core::ffi::c_int;

use rusl_core::c_types::{locale_t, wint_t};

/// 判断宽字符是否为可打印字符。
///
/// 可打印字符判定逻辑 (按优先级):
///
/// 1. `wc < 0xFF` 且 `(wc+1) & 0x7F >= 0x21` — ASCII 可打印热路径
/// 2. `wc < 0x2028` — BMP 排除 C1 控制字符后的码点
/// 3. `wc` 在 `[0x202A, 0xD7FF]` 或 `[0xE000, 0xFFF8]` — 排除行分隔符和代理区
/// 4. `wc >= 0xFFFC` — 排除非字符/替换字符
/// 5. `(wc & 0xFFFE) == 0xFFFE` — 排除所有 FFFE/FFFF 非字符码点
/// 6. 其余 — CJK 扩展及高位平面字符
///
/// 非可打印字符包括: C0/C1 控制字符、DEL、行/段分隔符、代理区、
/// 非字符码点、替换字符、行间注释锚点、WEOF。
///
/// # 安全性
///
/// 此函数标记为 `unsafe` 以保持 C ABI 签名兼容。
/// 实际调用无内存安全性风险。
#[no_mangle]
pub unsafe extern "C" fn iswprint(wc: wint_t) -> c_int {
    __iswprint_l(wc, core::ptr::null_mut())
}

/// locale 感知的可打印字符判断。
///
/// 在 C locale 下行为与 [`iswprint`] 等价。
///
/// # 安全性
///
/// - `l`: 必须为有效的 locale 句柄, 或 `NULL` 表示 C locale。
#[no_mangle]
pub unsafe extern "C" fn iswprint_l(wc: wint_t, l: locale_t) -> c_int {
    __iswprint_l(wc, l)
}

/// 内部实现函数。供 [`iswctype_l`] 及 [`iswgraph`] 内部实现调用。
///
/// [`iswprint`] 等价于 `__iswprint_l(wc, core::ptr::null_mut())`。
///
/// 采用五阶段决策树算法, 复刻 musl C 源码逻辑:
///
/// 1. `wc < 0xFF` 时: 位运算检查低 7 位是否在 [0x20, 0x7E]
///    `(wc+1) & 0x7F >= 0x21`, ASCII 可打印字符热路径
/// 2. `wc < 0x2028` 或 `wc` 在 [0x202A, 0xD7FF] 或 [0xE000, 0xFFF8]
///    排除 C1 控制字符和行分隔符/代理区
/// 3. `wc < 0xFFFC` 或 `wc > 0x10FFFF` 或 `(wc & 0xFFFE) == 0xFFFE`
///    排除非字符码点和越界码点 -> 返回 0
/// 4. 其余高位平面字符 -> 返回 1
///
/// O(1) 时间复杂度, 最坏情况 4 个分支。
///
/// `_l` 参数当前保留为 API 兼容占位, 内部实现仅依赖 C locale 行为。
///
/// # 安全性
///
/// 同 [`iswprint_l`]。
pub(crate) fn __iswprint_l(wc: wint_t, _l: locale_t) -> c_int {
    let w = wc as u32;

    // Phase 1: wc < 0xFF — ASCII 可打印字符热路径
    // (wc+1) & 0x7F >= 0x21 等价于低 7 位在 0x20..=0x7E 范围内
    // 自动排除: C0 控制字符 (0x00-0x1F), DEL (0x7F), C1 控制字符 (0x80-0x9F)
    if w < 0xff {
        return ((w + 1) & 0x7f >= 0x21) as c_int;
    }

    // Phase 2/3: BMP 可打印范围
    // - w < 0x2028: 覆盖 0xFF..0x2027 (NBSP 到各种符号)
    // - w-0x202a < 0xd800-0x202a: 即 w 在 [0x202A, 0xD7FF]
    // - w-0xe000 < 0xfff9-0xe000: 即 w 在 [0xE000, 0xFFF8]
    if w < 0x2028
        || w.wrapping_sub(0x202a) < (0xd800 - 0x202a)
        || w.wrapping_sub(0xe000) < (0xfff9 - 0xe000)
    {
        return 1;
    }

    // Phase 4/5: 排除非字符、替换字符和越界码点
    // w.wrapping_sub(0xfffc) > 0x10ffff-0xfffc:
    //   为真当 w < 0xFFFC (回绕为大值) 或 w > 0x10FFFF
    // (w & 0xFFFE) == 0xFFFE: 检测所有以 FFFE/FFFF 结尾的非字符码点
    if w.wrapping_sub(0xfffc) > (0x10ffff - 0xfffc) || (w & 0xfffe) == 0xfffe {
        return 0;
    }

    // Phase 6: 其余高位平面字符 (如 U+10000..U+10FFFD 除 U+xFFFE/U+xFFFF)
    1
}
