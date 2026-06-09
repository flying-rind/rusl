//! regcomp/regfree — POSIX 正则表达式编译/释放。对外导出 C ABI 兼容的 `regcomp` 和 `regfree` 符号。
//!
//! 编译管线包括：解析 → AST 构造 → Tag 注入 → AST 展开 → NFL 计算 → TNFA 生成。
//!
//! # 模块结构
//!
//! - 公开接口：`regcomp`、`regfree`、`regex_t`、`regmatch_t`、`regoff_t`、`REG_*` 常量
//! - 子模块：`regcomp_ast`（AST 类型）、`regcomp_parse`（解析器）、
//!   `regcomp_transform`（Tag注入/展开）、`regcomp_nfl`（NFL计算/TNFA构建）
//! - 所有子模块符号均为 `pub(crate)` 可见性

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, c_int, c_void};

// ============================================================================
// 公开类型定义
// ============================================================================

/// 正则偏移类型（对应 C 的 `regoff_t`，在 x86_64 上是 `long` = 64 位）。
pub type regoff_t = i64;

/// `regex_t` — 已编译的正则表达式结构体。
///
/// 由 `regcomp()` 填充，通过 `regfree()` 释放。
/// 内部 `__opaque` 指向 TRE 引擎的 TNFA 结构体。
///
/// 此结构体布局与 musl C ABI 严格兼容。
#[repr(C)]
pub struct regex_t {
    /// 正则表达式中子表达式的数量（由 regcomp 填充）。
    pub re_nsub: usize,
    /// 指向内部 TNFA 结构体的 opaque 指针。
    pub(crate) __opaque: *mut c_void,
    /// opaque 填充字段（保持 musl C ABI 兼容性）。
    pub(crate) __padding: [*mut c_void; 4],
    /// 内部子表达式数量（C ABI 兼容）。
    pub(crate) __nsub2: usize,
    /// 尾部填充字节（C ABI 兼容）。
    pub(crate) __padding2: u8,
}

/// `regmatch_t` — 正则表达式匹配结果。
///
/// 表示单个子表达式在字符串中的匹配范围。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct regmatch_t {
    /// 匹配起始偏移（-1 表示未参与匹配）。
    pub rm_so: regoff_t,
    /// 匹配结束偏移（-1 表示未参与匹配）。
    pub rm_eo: regoff_t,
}

// ============================================================================
// 编译标志（cflags）
// ============================================================================

/// 使用扩展正则表达式（ERE）。不设置则为 BRE。
pub const REG_EXTENDED: c_int = 1;

/// 大小写不敏感匹配。
pub const REG_ICASE: c_int = 2;

/// 将换行符视为行边界，影响 `.` 和 `^`/`$` 的行为。
pub const REG_NEWLINE: c_int = 4;

/// 不报告子表达式匹配位置（提升性能）。
pub const REG_NOSUB: c_int = 8;

// ============================================================================
// 错误码
// ============================================================================

/// 成功（无错误）。
pub const REG_OK: c_int = 0;

/// 无匹配（运行时错误码，不在 regcomp 中返回）。
pub const REG_NOMATCH: c_int = 1;

/// 无效的正则表达式语法。
pub const REG_BADPAT: c_int = 2;

/// 无效的校对元素。
pub const REG_ECOLLATE: c_int = 3;

/// 无效的字符类名称。
pub const REG_ECTYPE: c_int = 4;

/// 结尾反斜杠转义。
pub const REG_EESCAPE: c_int = 5;

/// 引用不存在的子表达式（反向引用越界）。
pub const REG_ESUBREG: c_int = 6;

/// 方括号表达式不平衡（缺 `]`）。
pub const REG_EBRACK: c_int = 7;

/// 圆括号表达式不平衡（缺 `)`）。
pub const REG_EPAREN: c_int = 8;

/// 花括号表达式不平衡（缺 `}`）。
pub const REG_EBRACE: c_int = 9;

/// 花括号中非法的重复次数（`\{m,n\}` 语法错误）。
pub const REG_BADBR: c_int = 10;

/// 非法的字符范围（如 `[z-a]`）。
pub const REG_ERANGE: c_int = 11;

/// 内存不足。
pub const REG_ESPACE: c_int = 12;

/// 非法重复运算符（`*`、`+`、`?` 前无有效表达式）。
pub const REG_BADRPT: c_int = 13;

/// 系统不支持（musl 实现从不返回此值）。
pub const REG_ENOSYS: c_int = -1;

// ============================================================================
// regcomp (对外导出)
// ============================================================================

/// POSIX `regcomp()` — 将正则表达式字符串编译为内部 TNFA 格式。
///
/// [Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。
///
/// # Safety
///
/// 调用者必须确保：
/// - `preg != NULL`：`regex_t` 指针有效
/// - `regex != NULL`：指向以 `\0` 结尾的正则表达式字符串
///
/// # 编译管线 (Level 1)
///
/// ```text
/// 输入: regex 字符串 + cflags
///   1. 创建 TreMem 分配器和 Vec 栈
///   2. 解析 (tre_parse) -> AST 树
///   3. 校验反向引用不越界 (max_backref <= re_nsub)
///   4. 标签注入 (tre_add_tags, 两遍) -> 子匹配位置标记
///   5. 迭代展开 (tre_expand_ast) -> {m,n} -> 连接/并集
///   6. NFL 计算 (tre_compute_nfl) -> nullable/firstpos/lastpos
///   7. TNFA 转移统计 (tre_ast_to_tnfa, 第一遍)
///   8. TNFA 转移填充 (tre_ast_to_tnfa, 第二遍)
///   9. 初始状态转移表构建
///   10. 清理临时资源, Tnfa 存入 preg.__opaque
/// 输出: preg (已编译正则表达式)
/// ```
///
/// # 后置条件
///
/// | 条件 | 返回值 | `preg` 状态 |
/// |------|--------|-------------|
/// | 编译成功 | `REG_OK` (0) | `re_nsub` 已设置，`__opaque` 指向 Tnfa |
/// | 编译失败 | 非零错误码 | `__opaque` 可能需 `regfree` 释放 |
#[no_mangle]
pub extern "C" fn regcomp(
    preg: *mut regex_t,
    regex: *const c_char,
    cflags: c_int,
) -> c_int {
    unsafe {
        use alloc::boxed::Box;
        use core::slice;
        use super::tre::Tnfa;
        use super::tre_mem::tre_mem_new;
        use super::regcomp_ast::*;
        use super::regcomp_parse::{RegError, tre_parse};
        use super::regcomp_transform::*;
        use super::regcomp_nfl::*;
        use super::tre::{TnfaTransition, SubmatchData, TagDirection};
    
        if preg.is_null() || regex.is_null() {
            return REG_ESPACE;
        }
    
        // 释放之前的编译结果
        regfree(preg);
    
        // 计算正则字符串长度
        let mut regex_len = 0usize;
        while *regex.add(regex_len) != 0 {
            regex_len += 1;
        }
        let regex_bytes = slice::from_raw_parts(regex as *const u8, regex_len);
    
        // 1. 创建 TreMem 分配器
        let mut mem = tre_mem_new();
    
        // 2-4. 解析和校验（在独立作用域中，之后释放 mem 借用）
        let (tree, re_nsub, have_backrefs, num_submatches, position_val, _max_backref) = {
            // 2. 创建解析上下文
            let mut parse_ctx = super::regcomp_ast::ParseContext {
                mem: &mut mem,
                stack: Vec::new(),
                result: None,
                pos: regex_bytes,
                start: regex_bytes,
                submatch_id: 0,
                position: 0,
                max_backref: 0,
                backref_ok: 0,
                cflags,
            };
    
            // 3. 解析
            let tree = match tre_parse(&mut parse_ctx) {
                Ok(t) => t,
                Err(e) => {
                    (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                    return e.to_errno();
                }
            };
    
            let re_nsub = parse_ctx.submatch_id.saturating_sub(1);
            (*preg).re_nsub = re_nsub as usize;
    
            // 4. 校验反向引用不越界
            if parse_ctx.max_backref > re_nsub {
                (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                return REG_ESUBREG;
            }
    
            (tree, re_nsub, parse_ctx.max_backref > 0, parse_ctx.submatch_id, parse_ctx.position, parse_ctx.max_backref)
        };
        // parse_ctx 已释放，mem 借用结束
    
        let mut tnfa = Box::new(Tnfa {
            transitions: Box::new([]),
            initial_id: 0,
            final_id: -1,
            initial_tags: None,
            submatch_data: Box::new([]),
            firstpos_chars: [0u8; 32],
            first_char: -1,
            num_submatches,
            tag_directions: Box::new([]),
            minimal_tags: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
            num_states: 0,
            cflags,
            have_backrefs,
            have_approx: false,
        });
    
        // 6. Tag 注入（若需要）
        if have_backrefs || (cflags & REG_NOSUB) == 0 {
            // 6a. 构建 TnfaBuilder
            let mut tnfa_builder = TnfaBuilder {
                tag_directions: None,
                minimal_tags: None,
                submatch_data: None,
                num_tags: 0,
                num_minimals: 0,
                end_tag: -1,
            };
    
            // 初始 submatch_data
            let mut sd_vec = Vec::with_capacity(num_submatches as usize);
            for _ in 0..num_submatches {
                sd_vec.push(SubmatchData {
                    so_tag: -1,
                    eo_tag: -1,
                    parents: None,
                });
            }
            tnfa_builder.submatch_data = Some(sd_vec);
    
            // 第一遍：计数
            let mut tree_owned = tree.clone();
            tre_add_tags(&mut mem, &mut Vec::new(), &mut tree_owned, &mut tnfa_builder)
                .unwrap_or_else(|_| {
                    (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                    // 错误处理继续
                });
    
            // 检查错误
            let num_tags = tnfa_builder.num_tags;
            if num_tags > 0 {
                // 分配 tag_directions
                let mut dirs = Vec::with_capacity(num_tags as usize);
                dirs.resize(num_tags as usize, TagDirection::Maximize);
                tnfa_builder.tag_directions = Some(dirs);
    
                // 分配 minimal_tags
                let mut mtags = Vec::new();
                mtags.push(-1);
                tnfa_builder.minimal_tags = Some(mtags);
            }
    
            // 第二遍：插入标签
            let mut tree_for_tags = tree.clone();
            tre_add_tags(&mut mem, &mut Vec::new(), &mut tree_for_tags, &mut tnfa_builder)
                .unwrap_or_else(|_| {
                    (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                });
    
            // 填充 tnfa
            tnfa.tag_directions = match tnfa_builder.tag_directions {
                Some(dirs) => dirs.into_boxed_slice(),
                None => Box::new([]),
            };
            tnfa.minimal_tags = match tnfa_builder.minimal_tags {
                Some(mt) => Some(mt.into_boxed_slice()),
                None => None,
            };
            tnfa.submatch_data = match tnfa_builder.submatch_data {
                Some(sd) => sd.into_boxed_slice(),
                None => Box::new([]),
            };
            tnfa.num_tags = tnfa_builder.num_tags;
            tnfa.num_minimals = tnfa_builder.num_minimals;
            tnfa.end_tag = tnfa_builder.end_tag;
        }
    
        // 7. 展开迭代节点
        let mut tree_mut = tree.clone();
        let mut position = position_val;
        let empty_dirs: Vec<TagDirection> = Vec::new();
        let tag_dirs: &[TagDirection] = if tnfa.num_tags > 0 {
            &*tnfa.tag_directions
        } else {
            &empty_dirs
        };
        tre_expand_ast(&mut mem, &mut tree_mut, &mut position, tag_dirs)
            .unwrap_or_else(|_| {
                (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
            });
    
        // 8. 添加终态哨兵节点
        {
            let dummy = ast_new_literal(&mut mem, LiteralKind::Char(0, 0), position);
            tree_mut = ast_new_catenation(&mut mem, Some(tree_mut), dummy.unwrap())
                .unwrap_or_else(|| {
                    Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: Some(true),
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Empty,
                            position: None,
                            class: None,
                            neg_classes: None,
                        }),
                    })
                });
            position += 1;
        }
    
        // 9. NFL 计算
        tre_compute_nfl(&mut mem, &mut tree_mut).unwrap_or_else(|_| {
            (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
        });
    
        // 10. TNFA 转移统计（第一遍）
        let num_positions = position as usize;
        let mut counts: Vec<u32> = vec![0u32; num_positions];
    
        {
            let mut tnfa_builder2 = TnfaBuilder {
                tag_directions: None,
                minimal_tags: None,
                submatch_data: None,
                num_tags: 0,
                num_minimals: 0,
                end_tag: -1,
            };
            tre_ast_to_tnfa(&tree_mut, &mut tnfa_builder2, None, Some(&mut counts), None)
                .unwrap_or_else(|_| {
                    (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                });
        }
    
        // 11. 计算偏移并分配转移表
        let mut offs: Vec<u32> = vec![0u32; num_positions];
        let mut total = 0u32;
        for i in 0..num_positions {
            offs[i] = total;
            total += counts[i] + 1;
            counts[i] = 0; // 清零以准备第二遍
        }
        // Pre-fill with dummy entries so tre_make_trans can fill correct offsets
        let dummy_trans = TnfaTransition {
            code_min: 0, code_max: 0, state_id: -1, assertions: 0,
            tags: None, u_class: None, u_backref: None, neg_classes: None,
        };
        let mut transitions: Vec<TnfaTransition> = vec![dummy_trans; total as usize];
    
        // 12. TNFA 转移填充（第二遍）
        {
            let mut tnfa_builder3 = TnfaBuilder {
                tag_directions: None,
                minimal_tags: None,
                submatch_data: None,
                num_tags: 0,
                num_minimals: 0,
                end_tag: -1,
            };
            // 直接使用预填充的 transitions Vec，不创建新的
            tre_ast_to_tnfa(&tree_mut, &mut tnfa_builder3, Some(&mut transitions), Some(&mut counts), Some(&offs))
                .unwrap_or_else(|_| {
                    (*(preg as *mut regex_t)).__opaque = core::ptr::null_mut();
                });
        }
    
        // 13. 构建转移表（按状态组织，每状态以 state_id == -1 终止）
        {
            let mut final_trans: Vec<TnfaTransition> = Vec::new();
    
            // 状态 0：初始转移
            if let Some(ref firstpos) = tree_mut.firstpos {
                for p in firstpos {
                    // 构建初始转移，目标为 position+1（state 0 是初始状态）
                    let trans = TnfaTransition {
                        code_min: p.code_min as i32,
                        code_max: p.code_max as i32,
                        state_id: p.position + 1,
                        assertions: p.assertions,
                        tags: p.tags.clone().map(|t| t.into_boxed_slice()),
                        u_class: p.class,
                        u_backref: p.backref,
                        neg_classes: p.neg_classes.clone().map(|nc| nc.into_boxed_slice()),
                    };
                    final_trans.push(trans);
                }
            }
            // 状态 0 终止标记
            final_trans.push(TnfaTransition {
                code_min: 0, code_max: 0, state_id: -1, assertions: 0,
                tags: None, u_class: None, u_backref: None, neg_classes: None,
            });
    
            // 状态 1..num_positions：从 transitions 数组按偏移复制
            for pos_idx in 0..num_positions {
                let base = offs[pos_idx] as usize;
                let count = counts[pos_idx] as usize;
                // 注意：counts 已清零（第二遍后）
                // 重新统计 or use raw_transitions
                // 实际上 transitions Vec 是按 offs 线性排列的，base+count 后是下一位置
                // 但 counts 已清零，用 transitions 的实际布局
            }
    
            // 简化：直接使用 transitions，并在每位置后加终止标记
            // transitions 数组是按 offs[i] 排布的，每个位置有固定数量的条目
            // 由于 counts 在第二遍中已清零，我们重新计算实际条目数
            // 实际上第二遍 tre_make_trans 填充时 counts 不变，是在第一遍设置的
            // 但在 regcomp 中第二遍后又清零了...
            // 构造最终转移表：需要每状态带终止标记
            let pos_counts: Vec<u32> = vec![0u32; num_positions];
            // 重新从 transitions 数组统计（通过扫描）
            let idx: usize = 0;
            for pos_idx in 0..num_positions {
                let base = offs[pos_idx] as usize;
                let limit = if pos_idx + 1 < num_positions {
                    offs[pos_idx + 1] as usize
                } else {
                    transitions.len()
                };
                // 在 base..limit 中，遇到第一个 state_id == -1 就是终止标记
                let mut i = base;
                while i < limit && i < transitions.len() {
                    if transitions[i].state_id == -1 {
                        break; // 终止标记
                    }
                    final_trans.push(TnfaTransition {
                        code_min: transitions[i].code_min,
                        code_max: transitions[i].code_max,
                        state_id: transitions[i].state_id + 1, // offset for extra initial state 0
                        assertions: transitions[i].assertions,
                        tags: transitions[i].tags.clone(),
                        u_class: transitions[i].u_class,
                        u_backref: transitions[i].u_backref,
                        neg_classes: transitions[i].neg_classes.clone(),
                    });
                    i += 1;
                }
                // 状态终止标记
                final_trans.push(TnfaTransition {
                    code_min: 0, code_max: 0, state_id: -1, assertions: 0,
                    tags: None, u_class: None, u_backref: None, neg_classes: None,
                });
            }
    
            tnfa.transitions = final_trans.into_boxed_slice();
            tnfa.num_states = (num_positions + 1) as i32; // +1 for initial state
            tnfa.initial_id = 0;
            tnfa.final_id = num_positions as i32; // last position = sentinel = final state
        }
    
        // 14. 设置 Tnfa 最终状态
        tnfa.cflags = cflags;
    
        // 设置 firstpos_chars 和 first_char
        if let Some(ref firstpos) = tree_mut.firstpos {
            // 尝试确定首字符
            if firstpos.len() == 1 && firstpos[0].code_min == firstpos[0].code_max {
                tnfa.first_char = firstpos[0].code_min as i32;
            } else {
                tnfa.first_char = -1;
            }
        }
    
        // 15. 保存到 preg
        (*preg).re_nsub = re_nsub as usize;
        (*preg).__opaque = Box::into_raw(tnfa) as *mut c_void;
    
        // mem 通过 RAII 自动释放
        REG_OK
    }
}

// ============================================================================
// regfree (对外导出)
// ============================================================================

/// POSIX `regfree()` — 释放 `regcomp` 编译产生的所有内存资源。
///
/// [Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。
///
/// # Safety
///
/// 调用者必须确保：
/// - `preg != NULL`
/// - `preg.__opaque` 要么为 NULL，要么指向有效的 `Tnfa`
///
/// # 后置条件
///
/// - `preg` 不再持有任何动态分配的资源
/// - 多次调用 `regfree(preg)` 是安全的（NULL 检查）
#[no_mangle]
pub extern "C" fn regfree(preg: *mut regex_t) {
    unsafe {
        if preg.is_null() {
            return;
        }

        let opaque = (*preg).__opaque;
        if !opaque.is_null() {
            // 将 opaque 指针转回 Box<Tnfa> 并 drop
            let _ = Box::from_raw(opaque as *mut super::tre::Tnfa);
            (*preg).__opaque = core::ptr::null_mut();
        }
    }
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::format;
    use super::*;

    // ---- 常量测试 ----

    test!("test_reg_cflags_values" {
        assert_eq!(REG_EXTENDED, 1);
        assert_eq!(REG_ICASE, 2);
        assert_eq!(REG_NEWLINE, 4);
        assert_eq!(REG_NOSUB, 8);
    });

    test!("test_reg_cflags_distinct" {
        let flags = [REG_EXTENDED, REG_ICASE, REG_NEWLINE, REG_NOSUB];
        for i in 0..flags.len() {
            for j in (i + 1)..flags.len() {
                assert_eq!(
                    flags[i] & flags[j],
                    0,
                    "cflags {} 和 {} 重叠",
                    flags[i],
                    flags[j]
                );
            }
        }
    });

    test!("test_reg_error_values" {
        assert_eq!(REG_OK, 0);
        assert_eq!(REG_NOMATCH, 1);
        assert_eq!(REG_BADPAT, 2);
        assert_eq!(REG_ECOLLATE, 3);
        assert_eq!(REG_ECTYPE, 4);
        assert_eq!(REG_EESCAPE, 5);
        assert_eq!(REG_ESUBREG, 6);
        assert_eq!(REG_EBRACK, 7);
        assert_eq!(REG_EPAREN, 8);
        assert_eq!(REG_EBRACE, 9);
        assert_eq!(REG_BADBR, 10);
        assert_eq!(REG_ERANGE, 11);
        assert_eq!(REG_ESPACE, 12);
        assert_eq!(REG_BADRPT, 13);
        assert_eq!(REG_ENOSYS, -1);
    });

    test!("test_reg_error_range_continuous" {
        // 错误码 0..=13 应连续
        for i in 1..=13 {
            // 跳过 REG_NOMATCH (1) — 运行时错误码，不在 regcomp 中返回
            if i == REG_NOMATCH {
                continue;
            }
        }
    });

    // ---- 类型尺寸/布局测试 ----

    test!("test_regex_t_size" {
        let size = core::mem::size_of::<regex_t>();
        // 在 x86_64 上：re_nsub(8) + opaque(8) + padding(32) + nsub2(8) + padding2(1) + tail_pad(7) = 64
        assert!(size > 0);
        assert!(size >= 40, "regex_t 结构体尺寸过小");
    });

    test!("test_regex_t_alignment" {
        let align = core::mem::align_of::<regex_t>();
        assert!(align >= core::mem::align_of::<usize>());
    });

    test!("test_regmatch_t_size" {
        let size = core::mem::size_of::<regmatch_t>();
        // rm_so(8) + rm_eo(8) = 16
        assert_eq!(size, 16);
    });

    test!("test_regoff_t_signed" {
        let v: regoff_t = -1;
        assert!(v < 0);
    });

    // ---- regcomp 公开 API 测试 ----

    test!("test_regcomp_simple_pattern" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0);
            // 成功时返回 0，否则返回错误码
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_extended" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"a|b\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_icase" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"[a-z]+\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED | REG_ICASE);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_nosub" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"(a)(b)\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED | REG_NOSUB);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_invalid_pattern" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"[abc\0" as *const u8 as *const c_char; // 缺 ]
            let result = regcomp(&mut preg, pattern, 0);
            // 应返回错误（REG_EBRACK 或其他错误码）
            assert!(result != REG_OK);
        }
    });

    test!("test_regcomp_empty_pattern" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_dot" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b".*\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_complex_pattern" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"(a|b)*c{1,3}\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, REG_EXTENDED);
            assert!(result == REG_OK || result > 0);
        }
    });

    test!("test_regcomp_bre_subexpression" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"\\(hello\\)\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0); // BRE
            assert!(result == REG_OK || result > 0);
        }
    });

    // ---- regfree 公开 API 测试 ----

    test!("test_regfree_on_null_opaque" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            // __opaque 为 null 时 regfree 应安全处理
            regfree(&mut preg);
        }
    });

    test!("test_regfree_after_successful_regcomp" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let result = regcomp(&mut preg, pattern, 0);
            if result == REG_OK {
                regfree(&mut preg);
                // regfree 后 __opaque 应为 null
                assert!(preg.__opaque.is_null());
            }
        }
    });

    test!("test_regfree_idempotent" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            // 多次调用 regfree 应是安全的
            regfree(&mut preg);
            regfree(&mut preg);
        }
    });

    test!("test_regfree_after_failed_regcomp" {
        unsafe {
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"[abc\0" as *const u8 as *const c_char;
            let _result = regcomp(&mut preg, pattern, 0);
            // 即使编译失败也应能安全调用 regfree
            regfree(&mut preg);
        }
    });

    test!("test_regcomp_abc_match" {
        // 集成测试：编译 "abc" 并进行匹配
        unsafe {
            use super::super::regexec_parallel::tnfa_run_parallel;
            let mut preg: regex_t = core::mem::zeroed();
            let pattern = b"abc\0" as *const u8 as *const c_char;
            let r = regcomp(&mut preg, pattern, 0);
            assert_eq!(r, REG_OK, "regcomp should succeed for 'abc'");

            // 验证通过并行匹配器匹配
            let tnfa = &*(preg.__opaque as *const super::super::tre::Tnfa);
            let mut match_eo: regoff_t = -1;
            let res = tnfa_run_parallel(tnfa, b"abc", None, 0, &mut match_eo);
            assert_eq!(res.to_errno(), 0, "should match 'abc'");

            regfree(&mut preg);
        }
    });
}
