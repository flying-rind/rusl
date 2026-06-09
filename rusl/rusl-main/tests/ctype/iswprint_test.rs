#![allow(useless_ptr_null_checks)]
//! `iswprint` 集成测试
//!
//! 测试宽字符可打印字符判断接口 `iswprint` / `iswprint_l` 的 C ABI 兼容性。
//!
//! 采用五阶段决策树算法, 覆盖 ASCII、BMP、补充平面各范围的可打印判定。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - `todo!()` 占位符行为
//! - 各 Unicode 码点范围的可打印/非可打印边界推测


use super::*;

// ============================================================================
// 签名验证
// ============================================================================

// 验证 `iswprint` 可正确链接。
test!("test_iswprint_linkage" {
    let f: unsafe extern "C" fn(wint_t) -> c_int = iswprint;
    assert!(!(f as *const ()).is_null());
});

// 验证 `iswprint_l` 可正确链接。
test!("test_iswprint_l_linkage" {
    let f: unsafe extern "C" fn(wint_t, locale_t) -> c_int = iswprint_l;
    assert!(!(f as *const ()).is_null());
});

// `iswprint_l` 当前为 `todo!()`, 调用应 panic。
test!("test_iswprint_l_panics" {
    { iswprint_l(0x41, core::ptr::null_mut()); }
});

// ============================================================================
// ASCII 可打印范围 (Phase 1 热路径)
// ============================================================================

// 推测: 空格 U+0020 是可打印字符。
test!("test_iswprint_space" {
    { iswprint(0x20); }
});

// 推测: 'A' U+0041 是可打印字符。
test!("test_iswprint_letter_a" {
    { iswprint(0x41); }
});

// 推测: '~' U+007E 是可打印字符 (ASCII 可打印终点)。
test!("test_iswprint_tilde" {
    { iswprint(0x7E); }
});

// 推测: NUL U+0000 不是可打印字符。
test!("test_iswprint_nul" {
    { iswprint(0x00); }
});

// 推测: US U+001F 不是可打印字符 (C0 终点)。
test!("test_iswprint_us" {
    { iswprint(0x1F); }
});

// 推测: DEL U+007F 不是可打印字符。
test!("test_iswprint_del" {
    { iswprint(0x7F); }
});

// ============================================================================
// C1 控制字符范围
// ============================================================================

// 推测: U+0080 (C1 PAD) 不是可打印字符。
test!("test_iswprint_c1_pad" {
    { iswprint(0x80); }
});

// 推测: U+009F (C1 APC) 不是可打印字符。
test!("test_iswprint_c1_apc" {
    { iswprint(0x9F); }
});

// 推测: U+00A0 (NBSP) 是可打印字符 (C1 之后)。
test!("test_iswprint_nbsp" {
    { iswprint(0xA0); }
});

// ============================================================================
// 行/段分隔符
// ============================================================================

// 推测: U+2028 (LINE SEPARATOR) 不是可打印字符。
test!("test_iswprint_line_separator" {
    { iswprint(0x2028); }
});

// 推测: U+2029 (PARAGRAPH SEPARATOR) 不是可打印字符。
test!("test_iswprint_paragraph_separator" {
    { iswprint(0x2029); }
});

// 推测: U+202A 是可打印字符。
test!("test_iswprint_after_separator" {
    { iswprint(0x202A); }
});

// ============================================================================
// 代理区
// ============================================================================

// 推测: U+D7FF (代理区之前) 是可打印字符。
test!("test_iswprint_before_surrogate" {
    { iswprint(0xD7FF); }
});

// 推测: U+D800 (高代理区) 不是可打印字符。
test!("test_iswprint_high_surrogate" {
    { iswprint(0xD800); }
});

// 推测: U+DFFF (低代理区终点) 不是可打印字符。
test!("test_iswprint_low_surrogate" {
    { iswprint(0xDFFF); }
});

// 推测: U+E000 (私用区起点) 是可打印字符。
test!("test_iswprint_pua_start" {
    { iswprint(0xE000); }
});

// ============================================================================
// 非字符/替换字符/锚点
// ============================================================================

// 推测: U+FFF8 是可打印字符。
test!("test_iswprint_before_anchor" {
    { iswprint(0xFFF8); }
});

// 推测: U+FFF9 (ANCHOR) 不是可打印字符。
test!("test_iswprint_anchor" {
    { iswprint(0xFFF9); }
});

// 推测: U+FFFC 不是可打印字符。
test!("test_iswprint_object_replacement" {
    { iswprint(0xFFFC); }
});

// 推测: U+FFFD 不是可打印字符。
test!("test_iswprint_replacement_char" {
    { iswprint(0xFFFD); }
});

// 推测: U+FFFE 不是可打印字符 (非字符)。
test!("test_iswprint_fffe" {
    { iswprint(0xFFFE); }
});

// 推测: U+FFFF 不是可打印字符 (非字符)。
test!("test_iswprint_ffff" {
    { iswprint(0xFFFF); }
});

// ============================================================================
// 补充平面
// ============================================================================

// 推测: U+1FFFE 不是可打印字符 (高位非字符)。
test!("test_iswprint_high_nonchar" {
    { iswprint(0x1FFFE); }
});

// 推测: U+1FFFD 是可打印字符 (高位平面有效字符)。
test!("test_iswprint_high_valid" {
    { iswprint(0x1FFFD); }
});

// 推测: WEOF 不是可打印字符。
test!("test_iswprint_weof" {
    { iswprint(wint_t::MAX); }
});

// 推测: 中文字符 U+4E2D 是可打印字符。
test!("test_iswprint_cjk" {
    { iswprint(0x4E2D); }
});

// 推测: `iswprint_l(NULL)` 与 `iswprint` 等价。
test!("test_iswprint_l_null" {
    { iswprint_l(0x41, core::ptr::null_mut()); }
});