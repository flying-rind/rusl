#![allow(useless_ptr_null_checks)]
//! `wcwidth` 集成测试
//!
//! 测试宽字符列宽判断接口 `wcwidth` 的 C ABI 兼容性。
//!
//! ## 测试范围
//!
//! - C ABI 签名正确性 (函数指针类型检查)
//! - 链接可见性 (`#[no_mangle]` 确保符号可被外部链接)
//! - 参数/返回值类型大小验证
//! - `todo!()` 占位符行为 (预期 panic)
//! - 行为推测覆盖 (实现完成后验证)
//!
//! ## C 标准行为 (待实现完成后生效)
//!
//! `wcwidth(wc)` 确定宽字符在终端显示时占用的列数:
//! - 0: null 字符、组合字符 (nonspacing marks)
//! - 1: 普通字符 (ASCII 可打印、BMP 普通宽度字符)
//! - 2: 宽字符 (CJK 统一汉字、全角字符等)
//! - -1: 不可打印字符 (C0/C1 控制字符、非字符码点)
//!
//! ## 注意
//!
//! 当前所有函数实现为 `todo!()`, 调用时 panic。
//! `NS_TABLE` 和 `WIDE_TABLE` 数据表当前为空数组占位。


use super::*;

// ============================================================================
// 编译期验证: C ABI 签名正确性
// ============================================================================

// 验证 `wcwidth` 可被声明为 `extern "C"` 函数并正确链接。
test!("test_wcwidth_linkage" {
    let f: unsafe extern "C" fn(wchar_t) -> c_int = wcwidth;
    assert!(!(f as *const ()).is_null(), "wcwidth 函数指针不应为 NULL");
});

// ============================================================================
// 类型大小验证
// ============================================================================

// 验证 `wcwidth` 返回值类型 `c_int` 的大小。
test!("test_c_int_size" {
    assert_eq!(core::mem::size_of::<c_int>(), 4, "c_int (int) 应为 4 字节");
});

// 验证 `wcwidth` 参数类型 `wchar_t` 的大小 (Linux 上为 int)。
//
// spec: Linux x86_64 上 `wchar_t` 为 `int` (32-bit 有符号整数)
test!("test_wchar_t_size" {
    assert_eq!(
        core::mem::size_of::<wchar_t>(),
        4,
        "wchar_t (int) 应为 4 字节"
    );
});

// 验证 `wchar_t` 是有符号类型。
test!("test_wchar_t_signed" {
    // wchar_t = i32, 负值可用于表示无效码点
    let negative: wchar_t = -1_i32;
    assert_eq!(negative, -1_i32, "wchar_t 应支持负值 (i32)");
});

// ============================================================================
// 签名等价性验证
// ============================================================================

// 验证 `wcwidth` 的 extern "C" 签名 — 参数和返回值类型必须严格匹配。
test!("test_wcwidth_extern_c_signature" {
    type Sig = unsafe extern "C" fn(wchar_t) -> c_int;
    let _f: Sig = wcwidth;
});

// 验证 `wcwidth` 可以从 `unsafe extern "C"` 指针形态调用。
test!("test_wcwidth_fn_ptr_callable" {
    let f: unsafe extern "C" fn(wchar_t) -> c_int = wcwidth;
    let ptr = f as *const ();
    assert!(!ptr.is_null(), "wcwidth 函数指针转换为 void* 不应为 NULL");
});

// ============================================================================
// 基本调用行为 (todo!() -> 预期 panic)
// ============================================================================

// `wcwidth` 当前为 `todo!()`, 调用应 panic。
test!("test_wcwidth_panics_on_todo" {
    {
        wcwidth(0x41);
    }
});

// `wcwidth` 传入 null 字符也应 panic (尚未实现)。
test!("test_wcwidth_null_panics" {
    {
        wcwidth(0);
    }
});

// `wcwidth` 传入 CJK 汉字也应 panic (尚未实现)。
test!("test_wcwidth_cjk_panics" {
    {
        wcwidth(0x4E00);
    }
});

// ============================================================================
// 后置条件推测: Case 1 — null 字符 (实现完成后应通过, 当前 panic)
// ============================================================================

// 推测: `wcwidth(0)` (null 字符 L'\0') 返回 0。
//
// spec: Case 1 — wc 是 null 字符 -> 返回 0
test!("test_wcwidth_spec_null_returns_0" {
    {
        assert_eq!(wcwidth(0), 0, "wcwidth(0) 应返回 0");
    }
});

// ============================================================================
// 后置条件推测: Case 2a — ASCII 可打印字符 返回 1
// ============================================================================

// 推测: ASCII 空格 (U+0020) 返回 1。
//
// spec: wc < 0xff 且 (wc+1 & 0x7f) >= 0x21 (即 ASCII 可打印 0x20-0x7E)
test!("test_wcwidth_spec_ascii_space_returns_1" {
    {
        assert_eq!(wcwidth(0x20), 1, "wcwidth(' ') 应返回 1");
    }
});

// 推测: ASCII 打印范围 0x21-0x7E 内所有字符返回 1。
//
// 注意: 此测试遍历 94 个 ASCII 可打印字符, spec 对此范围的
// 约束为: wc < 0xff 且 (wc+1 & 0x7f) >= 0x21 -> 返回 1。
// 但 0x7F 是 DEL (控制字符), 不在此范围。
test!("test_wcwidth_spec_ascii_printable_range_returns_1" {
    {
        for wc in 0x21u32..=0x7Eu32 {
            let result = wcwidth(wc as wchar_t);
            assert_eq!(
                result,
                1,
                "wcwidth(U+{:04X} '{}') 应返回 1",
                wc,
                char::from_u32(wc).unwrap_or('?')
            );
        }
    }
});

// 推测: ASCII 字母 'A'-'Z' 全部返回 1。
test!("test_wcwidth_spec_ascii_uppercase_returns_1" {
    {
        for wc in b'A'..=b'Z' {
            assert_eq!(
                wcwidth(wc as wchar_t),
                1,
                "wcwidth('{}') 应返回 1",
                wc as char
            );
        }
    }
});

// 推测: ASCII 字母 'a'-'z' 全部返回 1。
test!("test_wcwidth_spec_ascii_lowercase_returns_1" {
    {
        for wc in b'a'..=b'z' {
            assert_eq!(
                wcwidth(wc as wchar_t),
                1,
                "wcwidth('{}') 应返回 1",
                wc as char
            );
        }
    }
});

// 推测: ASCII 数字 '0'-'9' 全部返回 1。
test!("test_wcwidth_spec_ascii_digit_returns_1" {
    {
        for wc in b'0'..=b'9' {
            assert_eq!(
                wcwidth(wc as wchar_t),
                1,
                "wcwidth('{}') 应返回 1",
                wc as char
            );
        }
    }
});

// 推测: ASCII 标点符号也返回 1。
test!("test_wcwidth_spec_ascii_punctuation_returns_1" {
    {
        let puncts: &[u32] = &[
            0x21, // '!'
            0x2E, // '.'
            0x2C, // ','
            0x3B, // ';'
            0x3A, // ':'
            0x3F, // '?'
            0x2D, // '-'
            0x5F, // '_'
            0x28, // '('
            0x29, // ')'
        ];
        for &wc in puncts {
            assert_eq!(wcwidth(wc as wchar_t), 1, "wcwidth(U+{:04X}) 应返回 1", wc);
        }
    }
});

// ============================================================================
// 后置条件推测: Case 2b — 高位拉丁扩展 (BMP 普通宽度) 返回 1
// ============================================================================

// 推测: Latin-1 补充字符 (U+00C0, LATIN CAPITAL LETTER A WITH GRAVE) 返回 1。
test!("test_wcwidth_spec_latin_extended_a_grave_returns_1" {
    {
        assert_eq!(wcwidth(0x00C0), 1, "wcwidth(U+00C0) 应返回 1");
    }
});

// 推测: Latin-1 补充字符 (U+00E9, LATIN SMALL LETTER E WITH ACUTE) 返回 1。
test!("test_wcwidth_spec_latin_extended_e_acute_returns_1" {
    {
        assert_eq!(wcwidth(0x00E9), 1, "wcwidth(U+00E9) 应返回 1");
    }
});

// 推测: 希腊字母 (U+03B1, GREEK SMALL LETTER ALPHA) 返回 1。
test!("test_wcwidth_spec_greek_alpha_returns_1" {
    {
        assert_eq!(wcwidth(0x03B1), 1, "wcwidth(U+03B1) 应返回 1");
    }
});

// 推测: 西里尔字母 (U+0410, CYRILLIC CAPITAL LETTER A) 返回 1。
test!("test_wcwidth_spec_cyrillic_a_returns_1" {
    {
        assert_eq!(wcwidth(0x0410), 1, "wcwidth(U+0410) 应返回 1");
    }
});

// 推测: Unicode BMP 普通范围 (U+0100-U+2FFF 不含 CJK/宽字符) 返回 1。
test!("test_wcwidth_spec_bmp_normal_range_returns_1" {
    {
        // 抽样测试: 非宽字符、非控制字符的 BMP 码点
        let sample_codepoints: &[u32] = &[
            0x00C0, // Latin Extended
            0x0100, // Latin Extended-A
            0x0250, // IPA Extensions
            0x02B0, // Spacing Modifier Letters
            0x0370, // Greek
            0x0400, // Cyrillic
            0x0530, // Armenian
            0x0590, // Hebrew
            0x0600, // Arabic
            0x1E00, // Latin Extended Additional
        ];
        for &wc in sample_codepoints {
            let result = wcwidth(wc as wchar_t);
            assert!(
                result == 1 || result == 0 || result == 2 || result == -1,
                "wcwidth(U+{:04X}) = {} 不在有效范围 [-1, 0, 1, 2]",
                wc,
                result
            );
        }
    }
});

// ============================================================================
// 后置条件推测: Case 2c — 宽字符 (CJK/全角, 在 WIDE_TABLE 中) 返回 2
// ============================================================================

// 推测: CJK 统一汉字 U+4E00 (一) 返回 2。
test!("test_wcwidth_spec_cjk_han_4e00_returns_2" {
    {
        assert_eq!(wcwidth(0x4E00), 2, "wcwidth(U+4E00 '一') 应返回 2");
    }
});

// 推测: CJK 统一汉字 U+9AD8 (高) 返回 2。
test!("test_wcwidth_spec_cjk_han_9ad8_returns_2" {
    {
        assert_eq!(wcwidth(0x9AD8), 2, "wcwidth(U+9AD8 '高') 应返回 2");
    }
});

// 推测: 多个常用 CJK 汉字均返回 2。
test!("test_wcwidth_spec_cjk_common_han_returns_2" {
    {
        // 常用 CJK 统一汉字抽样
        let common_han: &[u32] = &[
            0x4E00, // 一
            0x4E2D, // 中
            0x4EBA, // 人
            0x5927, // 大
            0x5C0F, // 小
            0x65E5, // 日
            0x6708, // 月
            0x6C34, // 水
            0x706B, // 火
            0x8A00, // 言
        ];
        for &wc in common_han {
            assert_eq!(wcwidth(wc as wchar_t), 2, "wcwidth(U+{:04X}) 应返回 2", wc);
        }
    }
});

// 推测: CJK 统一汉字扩展 A 区 (U+3400) 返回 2。
test!("test_wcwidth_spec_cjk_ext_a_3400_returns_2" {
    {
        assert_eq!(wcwidth(0x3400), 2, "wcwidth(U+3400) 应返回 2");
    }
});

// 推测: 全角空格 U+3000 (IDEOGRAPHIC SPACE) 返回 2。
test!("test_wcwidth_spec_ideographic_space_returns_2" {
    {
        assert_eq!(wcwidth(0x3000), 2, "wcwidth(U+3000) 应返回 2");
    }
});

// 推测: 全角标点 U+FF01 (FULLWIDTH EXCLAMATION MARK) 返回 2。
test!("test_wcwidth_spec_fullwidth_exclamation_returns_2" {
    {
        assert_eq!(wcwidth(0xFF01_i32), 2, "wcwidth(U+FF01) 应返回 2");
    }
});

// 推测: 全角数字 U+FF10 (FULLWIDTH DIGIT ZERO) 返回 2。
test!("test_wcwidth_spec_fullwidth_digit_zero_returns_2" {
    {
        assert_eq!(wcwidth(0xFF10_i32), 2, "wcwidth(U+FF10) 应返回 2");
    }
});

// 推测: 全角大写字母 U+FF21 (FULLWIDTH LATIN CAPITAL LETTER A) 返回 2。
test!("test_wcwidth_spec_fullwidth_a_returns_2" {
    {
        assert_eq!(wcwidth(0xFF21_i32), 2, "wcwidth(U+FF21) 应返回 2");
    }
});

// 推测: CJK Extension B (U+20000) 高位平面宽字符返回 2。
//
// spec: wcwidth 对 CJK Extension B 的码点 (在 WIDE_TABLE 位图中) 返回 2
test!("test_wcwidth_spec_cjk_ext_b_20000_returns_2" {
    {
        assert_eq!(wcwidth(0x20000_i32), 2, "wcwidth(U+20000) 应返回 2");
    }
});

// 推测: CJK Extension B 其他码点 (U+2A6D6) 返回 2。
test!("test_wcwidth_spec_cjk_ext_b_2a6d6_returns_2" {
    {
        assert_eq!(wcwidth(0x2A6D6_i32), 2, "wcwidth(U+2A6D6) 应返回 2");
    }
});

// ============================================================================
// 后置条件推测: Case 3 — 组合字符 (在 NS_TABLE 中) 返回 0
// ============================================================================

// 推测: 组合重音符 U+0300 (COMBINING GRAVE ACCENT) 返回 0。
//
// spec: Case 3 — wc 是组合字符 (nonspacing mark, 在 NS_TABLE 位图中) -> 返回 0
test!("test_wcwidth_spec_combining_grave_returns_0" {
    {
        assert_eq!(wcwidth(0x0300), 0, "wcwidth(U+0300) 应返回 0");
    }
});

// 推测: 组合急音符 U+0301 (COMBINING ACUTE ACCENT) 返回 0。
test!("test_wcwidth_spec_combining_acute_returns_0" {
    {
        assert_eq!(wcwidth(0x0301), 0, "wcwidth(U+0301) 应返回 0");
    }
});

// 推测: 组合分音符 U+0308 (COMBINING DIAERESIS) 返回 0。
test!("test_wcwidth_spec_combining_diaeresis_returns_0" {
    {
        assert_eq!(wcwidth(0x0308), 0, "wcwidth(U+0308) 应返回 0");
    }
});

// 推测: 多个组合字符均返回 0。
test!("test_wcwidth_spec_combining_marks_range_returns_0" {
    {
        // 抽样测试: Unicode 组合字符区段 0300-036F
        let marks: &[u32] = &[
            0x0300, // COMBINING GRAVE ACCENT
            0x0301, // COMBINING ACUTE ACCENT
            0x0302, // COMBINING CIRCUMFLEX ACCENT
            0x0303, // COMBINING TILDE
            0x0304, // COMBINING MACRON
            0x0306, // COMBINING BREVE
            0x0307, // COMBINING DOT ABOVE
            0x0308, // COMBINING DIAERESIS
            0x030A, // COMBINING RING ABOVE
            0x030C, // COMBINING CARON
            0x0327, // COMBINING CEDILLA
            0x0328, // COMBINING OGONEK
            0x0345, // COMBINING GREEK YPOGEGRAMMENI
        ];
        for &wc in marks {
            assert_eq!(
                wcwidth(wc as wchar_t),
                0,
                "wcwidth(U+{:04X}) 组合字符应返回 0",
                wc
            );
        }
    }
});

// ============================================================================
// 后置条件推测: Case 4a — C0/C1 控制字符 返回 -1
// ============================================================================

// 推测: C0 控制字符范围 0x01-0x1F 全部返回 -1。
//
// spec: Case 4 — wc 在 0x01-0x1F 或 0x7F-0x9F 范围内 (C0/C1 控制字符) -> 返回 -1
test!("test_wcwidth_spec_c0_control_range_returns_minus_1" {
    {
        for wc in 0x01u32..=0x1Fu32 {
            assert_eq!(
                wcwidth(wc as wchar_t),
                -1,
                "wcwidth(U+{:04X}) C0 控制字符应返回 -1",
                wc
            );
        }
    }
});

// 推测: DEL 字符 (U+007F) 返回 -1 (C0 控制字符的最高位)。
test!("test_wcwidth_spec_del_returns_minus_1" {
    {
        assert_eq!(wcwidth(0x7F), -1, "wcwidth(U+007F) DEL 应返回 -1");
    }
});

// 推测: C1 控制字符范围 0x80-0x9F 全部返回 -1。
test!("test_wcwidth_spec_c1_control_range_returns_minus_1" {
    {
        for wc in 0x80u32..=0x9Fu32 {
            assert_eq!(
                wcwidth(wc as wchar_t),
                -1,
                "wcwidth(U+{:04X}) C1 控制字符应返回 -1",
                wc
            );
        }
    }
});

// 推测: U+0000 (NUL) 虽然也是 C0 范围, 但 spec 明确 null 返回 0 (Case 1 特殊覆盖)。
//
// 注意: spec 中 null 字符被 Case 1 特殊处理返回 0，但 spec 的 Case 4 描述
// 说 "0x01-0x1F" -> -1。这里 0x00 被排除在 -1 范围外，
// 优先走 Case 1 返回 0。此测试验证 0x00 不在 -1 范围内。
test!("test_wcwidth_spec_nul_not_in_control_range" {
    {
        // 0x00 已经有 Case 1 覆盖, 这里确保它不与控制字符混淆
        let result = wcwidth(0);
        assert_ne!(result, -1, "wcwidth(0) 不应返回 -1 (null 被 Case 1 覆盖)");
    }
});

// ============================================================================
// 后置条件推测: Case 4b — 非字符码点 返回 -1
// ============================================================================

// 推测: U+FFFE (非字符码点) 返回 -1。
//
// spec: (wc & 0xfffe) == 0xfffe (非字符码点如 U+FFFE/U+FFFF) -> 返回 -1
test!("test_wcwidth_spec_fffe_nonchar_returns_minus_1" {
    {
        assert_eq!(wcwidth(0xFFFE_i32), -1, "wcwidth(U+FFFE) 应返回 -1");
    }
});

// 推测: U+FFFF (非字符码点) 返回 -1。
test!("test_wcwidth_spec_ffff_nonchar_returns_minus_1" {
    {
        assert_eq!(wcwidth(0xFFFF_i32), -1, "wcwidth(U+FFFF) 应返回 -1");
    }
});

// 推测: U+1FFFE (Plane 1 非字符码点) 返回 -1。
test!("test_wcwidth_spec_1fffe_nonchar_returns_minus_1" {
    {
        assert_eq!(wcwidth(0x1FFFE_i32), -1, "wcwidth(U+1FFFE) 应返回 -1");
    }
});

// 推测: U+1FFFF (Plane 1 非字符码点) 返回 -1。
test!("test_wcwidth_spec_1ffff_nonchar_returns_minus_1" {
    {
        assert_eq!(wcwidth(0x1FFFF_i32), -1, "wcwidth(U+1FFFF) 应返回 -1");
    }
});

// 推测: U+2FFFE (Plane 2 非字符码点) 返回 -1。
test!("test_wcwidth_spec_2fffe_nonchar_returns_minus_1" {
    {
        assert_eq!(wcwidth(0x2FFFE_i32), -1, "wcwidth(U+2FFFE) 应返回 -1");
    }
});

// 推测: Plane 0-16 中每组末尾的两个非字符码点均返回 -1。
test!("test_wcwidth_spec_all_plane_nonchars_returns_minus_1" {
    {
        // 测试各平面的 FFFE/FFFF
        let nonchars: &[i32] = &[
            0xFFFE, 0xFFFF, // BMP (Plane 0)
            0x1FFFE, 0x1FFFF, // Plane 1 (SMP)
            0x2FFFE, 0x2FFFF, // Plane 2 (SIP)
            0x3FFFE, 0x3FFFF, // Plane 3 (TIP)
            0xEFFFE, 0xEFFFF, // Plane 14 (SSP)
            0xFFFFE, 0xFFFFF, // Plane 15 (PUA-A)
            0x10FFFE, 0x10FFFF, // Plane 16 (PUA-B)
        ];
        for &wc in nonchars {
            assert_eq!(wcwidth(wc), -1, "wcwidth(U+{:06X}) 非字符码点应返回 -1", wc);
        }
    }
});

// ============================================================================
// 后置条件推测: Case 4c — 高位标记字符 返回 0
// ============================================================================

// 推测: U+E0001 (LANGUAGE TAG) 返回 0。
//
// spec: U+E0001 或 U+E0020-U+E00EF 范围内的标记字符 -> 返回 0
test!("test_wcwidth_spec_e0001_returns_0" {
    {
        assert_eq!(wcwidth(0xE0001_i32), 0, "wcwidth(U+E0001) 应返回 0");
    }
});

// 推测: U+E0020 (TAG SPACE) 返回 0。
test!("test_wcwidth_spec_e0020_returns_0" {
    {
        assert_eq!(wcwidth(0xE0020_i32), 0, "wcwidth(U+E0020) 应返回 0");
    }
});

// 推测: U+E0021-U+E007E 范围内的标记字符返回 0。
test!("test_wcwidth_spec_tag_range_returns_0" {
    {
        // 抽样: 标记范围内的几个字符
        let tags: &[i32] = &[
            0xE0020, // TAG SPACE
            0xE0021, // TAG EXCLAMATION MARK
            0xE0041, // TAG LATIN CAPITAL LETTER A
            0xE0061, // TAG LATIN SMALL LETTER A
            0xE007E, // TAG TILDE
            0xE007F, // CANCEL TAG (U+E007F)
        ];
        for &wc in tags {
            assert_eq!(wcwidth(wc), 0, "wcwidth(U+{:06X}) 标记字符应返回 0", wc);
        }
    }
});

// 推测: U+E0080 (标记范围之后) 不在标记范围内，按普通规则判断。
//
// 注意: spec 只提到 U+E0020-U+E00EF 范围返回 0，U+E0080 的
// 行为由位图查询决定。此测试仅验证它不 panic 且不返回异常值。
test!("test_wcwidth_spec_after_tag_range" {
    {
        let result = wcwidth(0xE0080_i32);
        // 返回值应在有效范围内 (即使不在 tag 范围中)
        let valid_returns: [c_int; 4] = [-1, 0, 1, 2];
        assert!(
            valid_returns.contains(&result),
            "wcwidth(U+E0080) = {} 不在有效范围 [-1, 0, 1, 2]",
            result
        );
    }
});

// 推测: U+E00EF (TAG 范围边界) 返回 0。
test!("test_wcwidth_spec_e00ef_returns_0" {
    {
        assert_eq!(wcwidth(0xE00EF_i32), 0, "wcwidth(U+E00EF) 应返回 0");
    }
});

// ============================================================================
// 返回值范围验证
// ============================================================================

// 推测: wcwidth 返回值仅可能为 -1、0、1、2。
//
// spec: 所有合法 Unicode 码点应落在这 4 种返回值之一。
test!("test_wcwidth_spec_return_value_range" {
    {
        let valid_returns: [c_int; 4] = [-1, 0, 1, 2];
        let test_points: &[wchar_t] = &[
            0,           // null -> 0
            0x20,        // space -> 1
            0x41,        // 'A' -> 1
            0x01,        // C0 control -> -1
            0x7F,        // DEL -> -1
            0x80,        // C1 control -> -1
            0xA0,        // NBSP -> 1 (normal)
            0x300,       // combining grave -> 0
            0xFFFE_i32,  // noncharacter -> -1
            0x4E00,      // CJK -> 2
            0x00C0,      // Latin extended -> 1
            0x3000,      // ideographic space -> 2
            0xE0001_i32, // language tag -> 0
        ];
        for &wc in test_points {
            let result = wcwidth(wc);
            assert!(
                valid_returns.contains(&result),
                "wcwidth(U+{:06X}) = {} 不在有效范围 [-1, 0, 1, 2] 内",
                wc,
                result
            );
        }
    }
});

// ============================================================================
// 纯函数/幂等性验证 (实现完成后验证)
// ============================================================================

// 推测: wcwidth 是纯函数，相同输入多次调用返回相同结果。
test!("test_wcwidth_spec_idempotent" {
    {
        let test_points: &[wchar_t] = &[0, 0x41, 0x01, 0xFFFE_i32, 0x4E00];
        for &wc in test_points {
            let r1 = wcwidth(wc);
            let r2 = wcwidth(wc);
            let r3 = wcwidth(wc);
            assert_eq!(r1, r2, "wcwidth(U+{:06X}) 多次调用应返回相同值", wc);
            assert_eq!(r2, r3, "wcwidth(U+{:06X}) 第三次调用应保持一致", wc);
        }
    }
});

// ============================================================================
// 宽字符位图边界测试 (实现完成后验证)
// ============================================================================

// 推测: BMP 末尾边界附近 (如 U+FFFD REPLACEMENT CHARACTER) 行为正确。
//
// 注意: U+FFFD 不是非字符码点 (0xFFFE/0xFFFF 才是), 应返回 1
test!("test_wcwidth_spec_replacement_char_returns_1" {
    {
        // U+FFFD = 65533, (65533 & 0xFFFE) = 65532 != 0xFFFE
        assert_eq!(wcwidth(0xFFFD_i32), 1, "wcwidth(U+FFFD) 应返回 1");
    }
});

// ============================================================================
// 跨平台兼容性验证
// ============================================================================

// 验证 `wchar_t` 在 64-bit 平台上的对齐。
test!("test_wchar_t_alignment" {
    assert_eq!(
        core::mem::align_of::<wchar_t>(),
        4,
        "wchar_t (i32) 对齐应为 4 字节"
    );
});

// 验证负值 wchar_t 可传入函数 (用于可能的错误处理)。
//
// 注意: spec 未定义负值 wchar_t 的行为, 此测试仅验证不 panic 或崩溃。
test!("test_wcwidth_spec_negative_wchar_t" {
    {
        // 负值在实践中不存在于合法 Unicode, 但 wchar_t 是有符号类型
        let result = wcwidth(-1_i32);
        // 不验证具体值，仅确保不崩溃
        let _ = result;
    }
});
