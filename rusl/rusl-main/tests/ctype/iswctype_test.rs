#![allow(useless_ptr_null_checks)]
//! `iswctype` / `wctype` 集成测试
//!
//! 测试通用宽字符分类接口 `iswctype`、`wctype` 及 locale 变体的 C ABI 兼容性。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性
//! - WCTYPE_* 常量值验证 (1-12 连续)
//! - 基本调用行为验证
//! - 边界输入验证

use core::ffi::{c_char, c_int};
use super::*;

// ============================================================================
// WCTYPE_* 常量验证
// ============================================================================

// 验证 12 个 WCTYPE_* 常量值严格连续 (1-12)。
test!("test_wctype_constants_sequential" {
	assert_eq!(WCTYPE_ALNUM, 1);
	assert_eq!(WCTYPE_ALPHA, 2);
	assert_eq!(WCTYPE_BLANK, 3);
	assert_eq!(WCTYPE_CNTRL, 4);
	assert_eq!(WCTYPE_DIGIT, 5);
	assert_eq!(WCTYPE_GRAPH, 6);
	assert_eq!(WCTYPE_LOWER, 7);
	assert_eq!(WCTYPE_PRINT, 8);
	assert_eq!(WCTYPE_PUNCT, 9);
	assert_eq!(WCTYPE_SPACE, 10);
	assert_eq!(WCTYPE_UPPER, 11);
	assert_eq!(WCTYPE_XDIGIT, 12);
});

// 验证所有 WCTYPE_* 常量值互不相同。
test!("test_wctype_constants_unique" {
	let values = [
		WCTYPE_ALNUM, WCTYPE_ALPHA, WCTYPE_BLANK, WCTYPE_CNTRL,
		WCTYPE_DIGIT, WCTYPE_GRAPH, WCTYPE_LOWER, WCTYPE_PRINT,
		WCTYPE_PUNCT, WCTYPE_SPACE, WCTYPE_UPPER, WCTYPE_XDIGIT,
	];
	for i in 0..values.len() {
		for j in (i + 1)..values.len() {
			assert_ne!(values[i], values[j],
				"常量值 {} 和 {} 冲突: 索引 {} 和 {} 都是 {}",
				i + 1, j + 1, i, j, values[i]);
		}
	}
});

// 验证 WCTYPE_* 常量类型为 `wctype_t`。
test!("test_wctype_constant_types" {
	let _a: wctype_t = WCTYPE_ALNUM;
	let _b: wctype_t = WCTYPE_ALPHA;
	let _c: wctype_t = WCTYPE_BLANK;
	let _d: wctype_t = WCTYPE_CNTRL;
	let _e: wctype_t = WCTYPE_DIGIT;
	let _f: wctype_t = WCTYPE_GRAPH;
	let _g: wctype_t = WCTYPE_LOWER;
	let _h: wctype_t = WCTYPE_PRINT;
	let _i: wctype_t = WCTYPE_PUNCT;
	let _j: wctype_t = WCTYPE_SPACE;
	let _k: wctype_t = WCTYPE_UPPER;
	let _l: wctype_t = WCTYPE_XDIGIT;
});

// ============================================================================
// 签名验证
// ============================================================================

// 验证 `iswctype` 可正确链接。
test!("test_iswctype_linkage" {
	let f: unsafe extern "C" fn(wint_t, wctype_t) -> c_int = iswctype;
	assert!(!(f as *const ()).is_null());
});

// 验证 `wctype` 可正确链接。
test!("test_wctype_linkage" {
	let f: unsafe extern "C" fn(*const c_char) -> wctype_t = wctype;
	assert!(!(f as *const ()).is_null());
});

// 验证 `iswctype_l` 可正确链接。
test!("test_iswctype_l_linkage" {
	let f: unsafe extern "C" fn(wint_t, wctype_t, locale_t) -> c_int = iswctype_l;
	assert!(!(f as *const ()).is_null());
});

// 验证 `wctype_l` 可正确链接。
test!("test_wctype_l_linkage" {
	let f: unsafe extern "C" fn(*const c_char, locale_t) -> wctype_t = wctype_l;
	assert!(!(f as *const ()).is_null());
});

// ============================================================================
// 类型大小验证
// ============================================================================

// 验证 `wctype_t` 的大小 (unsigned long)。
test!("test_wctype_t_size" {
	let sz = core::mem::size_of::<wctype_t>();
	assert!(sz == 4 || sz == 8,
		"wctype_t (unsigned long) 在 32-bit 平台为 4 字节, 在 64-bit 平台为 8 字节, 实际: {}", sz);
});

// 验证 `iswctype` 返回 `c_int` 类型, 大小为 4 字节。
test!("test_iswctype_return_size" {
	assert_eq!(core::mem::size_of::<c_int>(), 4);
});

// ============================================================================
// 基本调用行为 (实现已完成, 验证返回值)
// ============================================================================

// `iswctype` 对字母 'A' 使用 ALPHA 分类返回非零。
test!("test_iswctype_alpha_match" {
	unsafe {
		let result = iswctype(0x41, WCTYPE_ALPHA);
		assert_ne!(result, 0, "iswctype('A', ALPHA) 应返回非零");
	}
});

// `iswctype` 对数字 '0' 使用 ALPHA 分类返回 0。
test!("test_iswctype_alpha_reject_digit" {
	unsafe {
		let result = iswctype(0x30, WCTYPE_ALPHA);
		assert_eq!(result, 0, "iswctype('0', ALPHA) 应返回 0");
	}
});

// `wctype("alpha")` 返回 WCTYPE_ALPHA (2)。
test!("test_wctype_alpha" {
	let name = b"alpha\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, WCTYPE_ALPHA, "wctype(\"alpha\") 应返回 WCTYPE_ALPHA (2)");
	}
});

// `iswctype_l` 在 C locale 下行为与 iswctype 一致。
test!("test_iswctype_l_behaviour" {
	unsafe {
		let result = iswctype_l(0x41, WCTYPE_ALPHA, core::ptr::null_mut());
		assert_ne!(result, 0, "iswctype_l('A', ALPHA, NULL) 应返回非零");
	}
});

// `wctype_l` 在 C locale 下行为与 wctype 一致。
test!("test_wctype_l_behaviour" {
	let name = b"alpha\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype_l(name, core::ptr::null_mut());
		assert_eq!(result, WCTYPE_ALPHA, "wctype_l(\"alpha\", NULL) 应返回 WCTYPE_ALPHA (2)");
	}
});

// ============================================================================
// 边界输入验证
// ============================================================================

// 无效分类标识符 0 应导致 `iswctype` 返回 0。
test!("test_iswctype_invalid_zero" {
	unsafe {
		let result = iswctype(0x41, 0);
		assert_eq!(result, 0, "无效分类标识符 0 应返回 0");
	}
});

// 超出范围的分类标识符 13 应返回 0。
test!("test_iswctype_out_of_range" {
	unsafe {
		let result = iswctype(0x41, 13);
		assert_eq!(result, 0, "超出范围的分类标识符应返回 0");
	}
});

// WEOF 在任何分类下返回 0。
test!("test_iswctype_weof" {
	unsafe {
		let result = iswctype(wint_t::MAX, WCTYPE_ALNUM);
		assert_eq!(result, 0, "WEOF 在任何分类下应返回 0");
	}
});

// `wctype("xdigit")` 返回 WCTYPE_XDIGIT (12)。
test!("test_wctype_xdigit" {
	let name = b"xdigit\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, WCTYPE_XDIGIT, "wctype(\"xdigit\") 应返回 WCTYPE_XDIGIT (12)");
	}
});

// `wctype("unknown")` 返回 0。
test!("test_wctype_unknown" {
	let name = b"unknown\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, 0, "未知分类名称应返回 0");
	}
});

// `wctype` 处理空字符串时返回 0。
test!("test_wctype_empty_string" {
	let name = b"\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, 0, "空字符串应返回 0");
	}
});

// `wctype("alnum")` 返回 WCTYPE_ALNUM (1)。
test!("test_wctype_alnum" {
	let name = b"alnum\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, WCTYPE_ALNUM, "wctype(\"alnum\") 应返回 WCTYPE_ALNUM (1)");
	}
});

// `wctype("graph")` 返回 WCTYPE_GRAPH (6)。
test!("test_wctype_graph" {
	let name = b"graph\0".as_ptr() as *const c_char;
	unsafe {
		let result = wctype(name);
		assert_eq!(result, WCTYPE_GRAPH, "wctype(\"graph\") 应返回 WCTYPE_GRAPH (6)");
	}
});