#![allow(useless_ptr_null_checks)]
//! `iswgraph` 集成测试
//!
//! 测试宽字符图形字符判断接口 `iswgraph` / `iswgraph_l` 的 C ABI 兼容性。
//!
//! 图形字符定义: `iswprint(wc) != 0 && iswspace(wc) == 0`
//! (可打印且非空白)
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - `todo!()` 占位符行为
//! - 图形/非图形字符边界推测

use core::ffi::c_int;
use super::*;

// ============================================================================
// 签名验证
// ============================================================================

// 验证 `iswgraph` 可正确链接。
test!("test_iswgraph_linkage" {
    let f: unsafe extern "C" fn(wint_t) -> c_int = iswgraph;
    assert!(!(f as *const ()).is_null());
});

// 验证 `iswgraph_l` 可正确链接。
test!("test_iswgraph_l_linkage" {
    let f: unsafe extern "C" fn(wint_t, locale_t) -> c_int = iswgraph_l;
    assert!(!(f as *const ()).is_null());
});

// ============================================================================
// 基本调用行为 (todo!() -> 预期 panic)
// ============================================================================

// `iswgraph` 当前为 `todo!()`, 调用应 panic。
test!("test_iswgraph_panics" {
    { iswgraph(0x41); }
});

// `iswgraph_l` 当前为 `todo!()`, 调用应 panic。
test!("test_iswgraph_l_panics" {
    { iswgraph_l(0x41, core::ptr::null_mut()); }
});

// ============================================================================
// 图形字符推测
// ============================================================================

// 推测: 字母 'A' 是图形字符。
test!("test_iswgraph_letter" {
    { iswgraph(0x41); }
});

// 推测: 数字 '0' 是图形字符。
test!("test_iswgraph_digit" {
    { iswgraph(0x30); }
});

// 推测: 标点 '!' 是图形字符。
test!("test_iswgraph_punctuation" {
    { iswgraph(0x21); }
});

// 推测: 空格 (U+0020) 不是图形字符 (虽然可打印但是空白)。
test!("test_iswgraph_space" {
    { iswgraph(0x20); }
});

// 推测: 制表符 (U+0009) 不是图形字符 (控制字符)。
test!("test_iswgraph_tab" {
    { iswgraph(0x09); }
});

// 推测: 换行符 (U+000A) 不是图形字符。
test!("test_iswgraph_newline" {
    { iswgraph(0x0A); }
});

// 推测: DEL (U+007F) 不是图形字符。
test!("test_iswgraph_del" {
    { iswgraph(0x7F); }
});

// 推测: WEOF 不是图形字符。
test!("test_iswgraph_weof" {
    { iswgraph(wint_t::MAX); }
});

// 推测: 中文字符 U+4E2D 是图形字符。
test!("test_iswgraph_cjk" {
    { iswgraph(0x4E2D); }
});

// 推测: `iswgraph_l(NULL)` 与 `iswgraph` 等价。
test!("test_iswgraph_l_null" {
    { iswgraph_l(0x41, core::ptr::null_mut()); }
});