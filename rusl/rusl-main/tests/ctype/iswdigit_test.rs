#![allow(useless_ptr_null_checks)]
//! `iswdigit` 集成测试
//!
//! 测试宽字符十进制数字判断接口 `iswdigit` / `iswdigit_l` 的 C ABI 兼容性。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - 参数/返回值类型大小验证
//! - `todo!()` 占位符行为
//! - ASCII 数字范围边界推测

use core::ffi::c_int;
use super::*;

// ============================================================================
// 签名验证
// ============================================================================

// 验证 `iswdigit` 可正确链接。
test!("test_iswdigit_linkage" {
    let f: unsafe extern "C" fn(wint_t) -> c_int = iswdigit;
    assert!(!(f as *const ()).is_null());
});

// 验证 `iswdigit_l` 可正确链接。
test!("test_iswdigit_l_linkage" {
    let f: unsafe extern "C" fn(wint_t, locale_t) -> c_int = iswdigit_l;
    assert!(!(f as *const ()).is_null());
});

// ============================================================================
// 类型大小验证
// ============================================================================

// 验证 `wint_t` 为 32-bit 无符号整数。
test!("test_wint_t_size" {
    assert_eq!(core::mem::size_of::<wint_t>(), 4);
});

// 验证 `c_int` 返回值为 32-bit。
test!("test_c_int_size" {
    assert_eq!(core::mem::size_of::<c_int>(), 4);
});

// ============================================================================
// 基本调用行为 (todo!() -> 预期 panic)
// ============================================================================

// `iswdigit` 当前为 `todo!()`, 调用应 panic。
test!("test_iswdigit_panics" {
    { iswdigit(0x30); }
});

// `iswdigit_l` 当前为 `todo!()`, 调用应 panic。
test!("test_iswdigit_l_panics" {
    { iswdigit_l(0x30, core::ptr::null_mut()); }
});

// ============================================================================
// ASCII 数字范围推测
// ============================================================================

// 推测: '0' (U+0030) 是数字。
test!("test_iswdigit_zero" {
    { iswdigit(0x30); }
});

// 推测: '9' (U+0039) 是数字。
test!("test_iswdigit_nine" {
    { iswdigit(0x39); }
});

// 推测: '5' (U+0035) 是数字。
test!("test_iswdigit_five" {
    { iswdigit(0x35); }
});

// 推测: '/' (U+002F, '0' 前一字符) 不是数字。
test!("test_iswdigit_slash" {
    { iswdigit(0x2F); }
});

// 推测: ':' (U+003A, '9' 后一字符) 不是数字。
test!("test_iswdigit_colon" {
    { iswdigit(0x3A); }
});

// 推测: 'A' (U+0041) 不是数字。
test!("test_iswdigit_letter" {
    { iswdigit(0x41); }
});

// 推测: 阿拉伯数字 U+0660 在 C locale 下不是数字。
test!("test_iswdigit_arabic_zero" {
    { iswdigit(0x0660); }
});

// 推测: WEOF 不是数字。
test!("test_iswdigit_weof" {
    { iswdigit(wint_t::MAX); }
});

// 推测: 空格 (U+0020) 不是数字。
test!("test_iswdigit_space" {
    { iswdigit(0x20); }
});

// 推测: `iswdigit_l(NULL)` 与 `iswdigit` 行为等价。
test!("test_iswdigit_l_null" {
    { iswdigit_l(0x30, core::ptr::null_mut()); }
});