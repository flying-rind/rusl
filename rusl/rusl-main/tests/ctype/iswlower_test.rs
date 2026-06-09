#![allow(useless_ptr_null_checks)]
//! `iswlower` 集成测试
//!
//! 测试宽字符小写字母判断接口 `iswlower` / `iswlower_l` 的 C ABI 兼容性。
//!
//! 通过 `towupper(wc) != wc` 反向推断: 有小写形式的字符即为小写字母。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - `todo!()` 占位符行为
//! - 大小写字母边界推测

use core::ffi::c_int;

use super::*;

// ============================================================================
// 签名验证
// ============================================================================

// 验证 `iswlower` 可正确链接。
test!("test_iswlower_linkage" {
    let f: unsafe extern "C" fn(wint_t) -> c_int = iswlower;
    assert!(!(f as *const ()).is_null());
});

// 验证 `iswlower_l` 可正确链接。
test!("test_iswlower_l_linkage" {
    let f: unsafe extern "C" fn(wint_t, locale_t) -> c_int = iswlower_l;
    assert!(!(f as *const ()).is_null());
});

// ============================================================================
// 基本调用行为 (todo!() -> 预期 panic)
// ============================================================================

// `iswlower` 当前为 `todo!()`, 调用应 panic。
test!("test_iswlower_panics" {
    { iswlower(0x61); }
});

// `iswlower_l` 当前为 `todo!()`, 调用应 panic。
test!("test_iswlower_l_panics" {
    { iswlower_l(0x61, core::ptr::null_mut()); }
});

// ============================================================================
// 小写字母推测
// ============================================================================

// 推测: 'a' (U+0061) 是小写字母。
test!("test_iswlower_a" {
    { iswlower(0x61); }
});

// 推测: 'z' (U+007A) 是小写字母。
test!("test_iswlower_z" {
    { iswlower(0x7A); }
});

// 推测: 'm' (U+006D) 是小写字母。
test!("test_iswlower_m" {
    { iswlower(0x6D); }
});

// 推测: 'A' (U+0041) 不是小写字母 (是大写字母)。
test!("test_iswlower_uppercase_a" {
    { iswlower(0x41); }
});

// 推测: 'Z' (U+005A) 不是小写字母。
test!("test_iswlower_uppercase_z" {
    { iswlower(0x5A); }
});

// 推测: 数字 '1' (U+0031) 不是小写字母。
test!("test_iswlower_digit" {
    { iswlower(0x31); }
});

// 推测: 空格 (U+0020) 不是小写字母。
test!("test_iswlower_space" {
    { iswlower(0x20); }
});

// 推测: WEOF 不是小写字母 (towupper(WEOF) == WEOF)。
test!("test_iswlower_weof" {
    { iswlower(wint_t::MAX); }
});

// 推测: Unicode 小写字母 U+00E8 (e-grave) 是小写字母。
test!("test_iswlower_unicode_lower" {
    { iswlower(0x00E8); }
});

// 推测: Unicode 大写字母 U+00C8 (E-grave) 不是小写字母。
test!("test_iswlower_unicode_upper" {
    { iswlower(0x00C8); }
});

// 推测: `iswlower_l(NULL)` 与 `iswlower` 等价。
test!("test_iswlower_l_null" {
    { iswlower_l(0x61, core::ptr::null_mut()); }
});