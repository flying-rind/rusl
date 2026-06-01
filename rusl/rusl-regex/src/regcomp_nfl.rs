//! regcomp NFL 计算和 TNFA 构建 — Nullable/Firstpos/Lastpos 计算和 AST 到 TNFA 转换。
//!
//! 本模块包含：
//! - 位置集合操作：`tre_set_empty`、`tre_set_one`、`tre_set_union`
//! - 可空路径计算：`tre_match_empty`
//! - NFL 计算：`tre_compute_nfl`
//! - TNFA 转移构建：`tre_make_trans`
//! - AST 到 TNFA 转换：`tre_ast_to_tnfa`
//!
//! 所有符号均为 `pub(crate)` 可见性。

#![allow(unused_imports, unused_variables)]

use alloc::vec;
use alloc::vec::Vec;
use super::regcomp_ast::*;
use super::regcomp_parse::RegError;
use super::regcomp_transform::TnfaBuilder;
use super::tre::{TnfaTransition, TreCtype, TreCint, END_STATE_MARKER};
use super::tre_mem::TreMem;

// ============================================================================
// tre_set_empty — 空位置集合
// ============================================================================

/// 返回一个空的位置-标签集合。
pub(crate) fn tre_set_empty() -> Vec<PosAndTags> {
    Vec::new()
}

// ============================================================================
// tre_set_one — 单元素位置集合
// ============================================================================

/// 创建包含单一位置的位置-标签集合。
pub(crate) fn tre_set_one(
    _mem: &mut TreMem,
    pos: i32,
    code_min: i64,
    code_max: i64,
    tags: Option<Vec<i32>>,
    assertions: i32,
    class: Option<TreCtype>,
    neg_classes: Option<Vec<TreCtype>>,
    backref: Option<i32>,
) -> Vec<PosAndTags> {
    // C 中以 -1 position 作为终止标记；Rust 使用 Vec 的长度管理
    let pt = PosAndTags {
        position: pos,
        code_min,
        code_max,
        tags,
        assertions,
        class,
        neg_classes,
        backref,
    };
    vec![pt]
}

// ============================================================================
// tre_set_union — 位置集合并集
// ============================================================================

/// 计算两个位置-标签集合的并集。
pub(crate) fn tre_set_union(
    _mem: &mut TreMem,
    set1: &[PosAndTags],
    set2: &[PosAndTags],
    extra_tags: Option<&[i32]>,
    extra_assertions: i32,
) -> Vec<PosAndTags> {
    let mut result = Vec::with_capacity(set1.len() + set2.len());

    // 添加 set1 的元素（附加 extra_tags 和 extra_assertions）
    for p in set1 {
        let mut new_p = p.clone();
        new_p.assertions |= extra_assertions;
        if let Some(et) = extra_tags {
            if let Some(ref mut existing) = new_p.tags {
                if !existing.contains(&-1) {
                    existing.push(-1);
                }
                let idx = existing.iter().position(|&t| t < 0).unwrap_or(existing.len());
                // Merge extra_tags into existing
                for &t in et {
                    if t < 0 { break; }
                    if !existing.contains(&t) {
                        existing.insert(idx, t);
                    }
                }
            } else {
                let mut tags: Vec<i32> = Vec::new();
                for &t in et {
                    if t < 0 { break; }
                    tags.push(t);
                }
                new_p.tags = Some(tags);
            }
        }
        result.push(new_p);
    }

    // 添加 set2 的元素
    for p in set2 {
        result.push(p.clone());
    }

    result
}

// ============================================================================
// tre_match_empty — 可空路径计算
// ============================================================================

/// 遍历 AST 寻找可匹配空串的路径，收集路径上的 TAG 和 ASSERTION。
pub(crate) fn tre_match_empty(
    node: &AstNode,
    tags: &mut Vec<i32>,
    assertions: &mut i32,
    num_tags_seen: &mut u32,
) -> Result<(), RegError> {
    // 使用递归栈遍历 AST
    fn recurse(
        node: &AstNode,
        tags: &mut Vec<i32>,
        assertions: &mut i32,
        num_tags_seen: &mut u32,
    ) -> Result<(), RegError> {
        match node.node_type {
            AstType::Literal => {
                if let AstNodeObj::Literal(lit) = &node.obj {
                    match &lit.kind {
                        LiteralKind::Tag(tag_id) if *tag_id >= 0 => {
                            // 收集 tag 到 tags 列表（去重）
                            if !tags.contains(tag_id) {
                                // 找到第一个 -1 的位置
                                let mut pos = tags.len();
                                for (i, &t) in tags.iter().enumerate() {
                                    if t < 0 {
                                        pos = i;
                                        break;
                                    }
                                }
                                if pos >= tags.len() {
                                    tags.push(*tag_id);
                                    tags.push(-1);
                                } else {
                                    tags[pos] = *tag_id;
                                    if pos + 1 >= tags.len() {
                                        tags.push(-1);
                                    }
                                }
                            }
                            *num_tags_seen += 1;
                        }
                        LiteralKind::Assertion(a) => {
                            *assertions |= *a;
                        }
                        LiteralKind::Empty => {
                            // nothing
                        }
                        _ => {
                            // 普通字符：不收集，路径不可达空
                            // C 中 assert(0) — 这里不该调用
                        }
                    }
                }
            }
            AstType::Union => {
                if let AstNodeObj::Union(uni) = &node.obj {
                    // 左优先：如果左子可空，走左子；否则走右子
                    if uni.left.nullable == Some(true) {
                        recurse(&uni.left, tags, assertions, num_tags_seen)?;
                    } else if uni.right.nullable == Some(true) {
                        recurse(&uni.right, tags, assertions, num_tags_seen)?;
                    }
                }
            }
            AstType::Catenation => {
                if let AstNodeObj::Catenation(cat) = &node.obj {
                    // 必须经过两个子
                    recurse(&cat.left, tags, assertions, num_tags_seen)?;
                    recurse(&cat.right, tags, assertions, num_tags_seen)?;
                }
            }
            AstType::Iteration => {
                if let AstNodeObj::Iteration(iter) = &node.obj {
                    // 如果子可空，走子（空匹配优先）
                    if iter.arg.nullable == Some(true) {
                        recurse(&iter.arg, tags, assertions, num_tags_seen)?;
                    }
                }
            }
        }
        Ok(())
    }

    if node.nullable == Some(true) {
        recurse(node, tags, assertions, num_tags_seen)?;
    }
    Ok(())
}

// ============================================================================
// tre_compute_nfl — Nullable/Firstpos/Lastpos 计算
// ============================================================================

/// 对 AST 每个节点计算 `nullable`、`firstpos`、`lastpos` 属性。
pub(crate) fn tre_compute_nfl(
    mem: &mut TreMem,
    tree: &mut AstNode,
) -> Result<(), RegError> {
    compute_nfl_recursive(mem, tree)
}

fn compute_nfl_recursive(
    mem: &mut TreMem,
    node: &mut AstNode,
) -> Result<(), RegError> {
    match node.node_type {
        AstType::Literal => {
            let (nullable, firstpos, lastpos) = compute_nfl_literal(mem, node)?;
            node.nullable = Some(nullable);
            node.firstpos = Some(firstpos);
            node.lastpos = Some(lastpos);
        }
        AstType::Union => {
            // 递归处理左右子
            {
                let uni = match &mut node.obj {
                    AstNodeObj::Union(ref mut u) => u,
                    _ => return Err(RegError::BadPat),
                };
                compute_nfl_recursive(mem, &mut uni.left)?;
                compute_nfl_recursive(mem, &mut uni.right)?;
            }

            let (left_nullable, right_nullable, left_first, right_first, left_last, right_last) = {
                let uni = match &node.obj {
                    AstNodeObj::Union(ref u) => u,
                    _ => return Err(RegError::BadPat),
                };
                (uni.left.nullable.unwrap_or(false),
                 uni.right.nullable.unwrap_or(false),
                 uni.left.firstpos.clone().unwrap_or_default(),
                 uni.right.firstpos.clone().unwrap_or_default(),
                 uni.left.lastpos.clone().unwrap_or_default(),
                 uni.right.lastpos.clone().unwrap_or_default())
            };

            node.nullable = Some(left_nullable || right_nullable);
            node.firstpos = Some(tre_set_union(mem, &left_first, &right_first, None, 0));
            node.lastpos = Some(tre_set_union(mem, &left_last, &right_last, None, 0));
        }
        AstType::Iteration => {
            {
                let iter = match &mut node.obj {
                    AstNodeObj::Iteration(ref mut i) => i,
                    _ => return Err(RegError::BadPat),
                };
                compute_nfl_recursive(mem, &mut iter.arg)?;
            }

            let (min, arg_nullable, arg_first, arg_last) = {
                let iter = match &node.obj {
                    AstNodeObj::Iteration(ref i) => i,
                    _ => return Err(RegError::BadPat),
                };
                (iter.min, iter.arg.nullable.unwrap_or(false),
                 iter.arg.firstpos.clone().unwrap_or_default(),
                 iter.arg.lastpos.clone().unwrap_or_default())
            };

            node.nullable = Some(min == 0 || arg_nullable);
            node.firstpos = Some(arg_first);
            node.lastpos = Some(arg_last);
        }
        AstType::Catenation => {
            {
                let cat = match &mut node.obj {
                    AstNodeObj::Catenation(ref mut c) => c,
                    _ => return Err(RegError::BadPat),
                };
                compute_nfl_recursive(mem, &mut cat.left)?;
                compute_nfl_recursive(mem, &mut cat.right)?;
            }

            let (left_nullable, right_nullable, left_first, right_first, left_last, right_last) = {
                let cat = match &node.obj {
                    AstNodeObj::Catenation(ref c) => c,
                    _ => return Err(RegError::BadPat),
                };
                (cat.left.nullable.unwrap_or(false),
                 cat.right.nullable.unwrap_or(false),
                 cat.left.firstpos.clone().unwrap_or_default(),
                 cat.right.firstpos.clone().unwrap_or_default(),
                 cat.left.lastpos.clone().unwrap_or_default(),
                 cat.right.lastpos.clone().unwrap_or_default())
            };

            node.nullable = Some(left_nullable && right_nullable);

            // Compute firstpos
            node.firstpos = if left_nullable {
                // 左子可空：firstpos = right.firstpos ∪ left.firstpos (带左子的空路径 tags)
                let mut tags: Vec<i32> = Vec::new();
                tags.push(-1);
                let mut assertions: i32 = 0;
                let mut num_tags_seen: u32 = 0;
                if let AstNodeObj::Catenation(ref cat) = node.obj {
                    tre_match_empty(&cat.left, &mut tags, &mut assertions, &mut num_tags_seen)?;
                }
                // 过滤有效 tags
                let tag_slice: Vec<i32> = tags.iter().take_while(|&&t| t >= 0).copied().collect();
                if tag_slice.is_empty() {
                    Some(tre_set_union(mem, &right_first, &left_first, None, assertions))
                } else {
                    let mut tag_with_term = tag_slice;
                    tag_with_term.push(-1);
                    Some(tre_set_union(mem, &right_first, &left_first, Some(&tag_with_term), assertions))
                }
            } else {
                Some(left_first)
            };

            // Compute lastpos
            node.lastpos = if right_nullable {
                let mut tags: Vec<i32> = Vec::new();
                tags.push(-1);
                let mut assertions: i32 = 0;
                let mut num_tags_seen: u32 = 0;
                if let AstNodeObj::Catenation(ref cat) = node.obj {
                    tre_match_empty(&cat.right, &mut tags, &mut assertions, &mut num_tags_seen)?;
                }
                let tag_slice: Vec<i32> = tags.iter().take_while(|&&t| t >= 0).copied().collect();
                if tag_slice.is_empty() {
                    Some(tre_set_union(mem, &left_last, &right_last, None, assertions))
                } else {
                    let mut tag_with_term = tag_slice;
                    tag_with_term.push(-1);
                    Some(tre_set_union(mem, &left_last, &right_last, Some(&tag_with_term), assertions))
                }
            } else {
                Some(right_last)
            };
        }
    }
    Ok(())
}

// LITERAL 节点的 NFL 计算
fn compute_nfl_literal(
    mem: &mut TreMem,
    node: &AstNode,
) -> Result<(bool, Vec<PosAndTags>, Vec<PosAndTags>), RegError> {
    let lit = match &node.obj {
        AstNodeObj::Literal(l) => l,
        _ => return Err(RegError::BadPat),
    };

    match &lit.kind {
        LiteralKind::Backref(ref_val) => {
            // 反向引用：nullable = false, firstpos = {position}, lastpos = {position}
            let pos = lit.position.unwrap_or(0) as i32;
            let first = tre_set_one(mem, pos, 0, super::tre::TRE_CHAR_MAX as i64,
                None, 0, None, None, None);
            let last = tre_set_one(mem, pos, 0, super::tre::TRE_CHAR_MAX as i64,
                None, 0, None, None, Some(*ref_val));
            Ok((false, first, last))
        }
        LiteralKind::Empty | LiteralKind::Assertion(_) | LiteralKind::Tag(_) => {
            // 特殊节点：nullable = true, firstpos = {}, lastpos = {}
            Ok((true, tre_set_empty(), tre_set_empty()))
        }
        LiteralKind::Char(code_min_val, code_max_val) => {
            // 普通字面量：nullable = false, firstpos = {position}, lastpos = {position}
            let pos = lit.position.unwrap_or(0) as i32;
            let code_min = *code_min_val;
            let code_max = *code_max_val;
            let first = tre_set_one(mem, pos, code_min, code_max,
                None, 0, lit.class, lit.neg_classes.clone(), None);
            let last = tre_set_one(mem, pos, code_min, code_max,
                None, 0, lit.class, lit.neg_classes.clone(), None);
            Ok((false, first, last))
        }
    }
}

// ============================================================================
// tre_make_trans — 创建位置间转移
// ============================================================================

/// 从 `p1` 每个位置向 `p2` 每个位置创建转移边。
pub(crate) fn tre_make_trans(
    p1: &[PosAndTags],
    p2: &[PosAndTags],
    transitions: &mut Vec<TnfaTransition>,
    counts: &mut [u32],
    offs: &[u32],
) -> Result<(), RegError> {
    if transitions.is_empty() {
        // 第一遍：统计出边数量
        for p1_item in p1 {
            let p1_pos = p1_item.position as usize;
            if p1_pos < counts.len() {
                counts[p1_pos] += p2.len() as u32;
            }
        }
    } else {
        // 第二遍：填充转移表
        for p1_item in p1 {
            let p1_pos = p1_item.position as usize;
            if p1_pos >= offs.len() {
                continue;
            }
            let base = offs[p1_pos] as usize;

            for p2_item in p2 {
                // 在 base 位置找到下一个空闲转移槽
                let mut slot = base;
                loop {
                    if slot >= transitions.len() {
                        // 添加新转移
                        let mut trans = TnfaTransition {
                            code_min: 0,
                            code_max: 0,
                            state_id: -1,
                            assertions: 0,
                            tags: None,
                            u_class: None,
                            u_backref: None,
                            neg_classes: None,
                        };
                        // 填充内容
                        fill_transition(p1_item, p2_item, &mut trans)?;
                        transitions.push(trans);
                        break;
                    }
                    if transitions[slot].state_id == -1 || transitions[slot].state_id == END_STATE_MARKER {
                        // 未使用槽
                        fill_transition(p1_item, p2_item, &mut transitions[slot])?;
                        break;
                    }
                    slot += 1;
                }
            }
        }
    }
    Ok(())
}

fn fill_transition(
    p1: &PosAndTags,
    p2: &PosAndTags,
    trans: &mut TnfaTransition,
) -> Result<(), RegError> {
    trans.code_min = p1.code_min as i32;
    trans.code_max = p1.code_max as i32;
    trans.state_id = p2.position;
    trans.assertions = p1.assertions | p2.assertions
        | (if p1.class.is_some() { super::tre::ASSERT_CHAR_CLASS } else { 0 })
        | (if p1.neg_classes.is_some() { super::tre::ASSERT_CHAR_CLASS_NEG } else { 0 });

    if let Some(backref) = p1.backref {
        if backref >= 0 {
            trans.u_backref = Some(backref);
            trans.assertions |= super::tre::ASSERT_BACKREF;
            trans.u_class = None;
        }
    } else {
        trans.u_class = p1.class;
    }

    // 合并 tags
    let mut combined_tags: Vec<i32> = Vec::new();
    if let Some(ref t1) = p1.tags {
        for &t in t1 {
            if t < 0 { break; }
            if !combined_tags.contains(&t) {
                combined_tags.push(t);
            }
        }
    }
    if let Some(ref t2) = p2.tags {
        for &t in t2 {
            if t < 0 { break; }
            if !combined_tags.contains(&t) {
                combined_tags.push(t);
            }
        }
    }
    if !combined_tags.is_empty() {
        combined_tags.push(-1);
        trans.tags = Some(combined_tags.into_boxed_slice());
    }

    // 复制 neg_classes
    if let Some(ref nc) = p1.neg_classes {
        trans.neg_classes = Some(nc.clone().into_boxed_slice());
    }

    Ok(())
}

// ============================================================================
// tre_ast_to_tnfa — AST 到 TNFA 编译
// ============================================================================

/// AST 到 TNFA 编译（递归，通过 firstpos/lastpos 创建转移边）。
///
/// 转换分为两遍：
/// - 第一遍：transitions 为 None → 统计出边计数
/// - 第二遍：transitions 为 Some → 填充转移边
///
/// transitions: 转移边数组（None = 第一遍计数模式）
/// counts: 每位置出边计数
/// offs: 每位置转移偏移
pub(crate) fn tre_ast_to_tnfa(
    node: &AstNode,
    tnfa: &mut TnfaBuilder,
    mut transitions: Option<&mut Vec<TnfaTransition>>,
    mut counts: Option<&mut [u32]>,
    offs: Option<&[u32]>,
) -> Result<(), RegError> {
    match node.node_type {
        AstType::Literal => {
            // 无操作：LITERAL 节点的转移由父节点处理
        }
        AstType::Union => {
            let uni = match &node.obj {
                AstNodeObj::Union(ref u) => u,
                _ => return Err(RegError::BadPat),
            };
            tre_ast_to_tnfa(&uni.left, tnfa, transitions.as_deref_mut(), counts.as_deref_mut(), offs)?;
            tre_ast_to_tnfa(&uni.right, tnfa, transitions.as_deref_mut(), counts.as_deref_mut(), offs)?;
        }
        AstType::Catenation => {
            let cat = match &node.obj {
                AstNodeObj::Catenation(ref c) => c,
                _ => return Err(RegError::BadPat),
            };
            // 创建从 left.lastpos → right.firstpos 的转移
            let left_last = cat.left.lastpos.as_deref().unwrap_or(&[]);
            let right_first = cat.right.firstpos.as_deref().unwrap_or(&[]);

            if let (Some(ref mut trans), Some(ref mut cnts), Some(ofs)) =
                (transitions.as_deref_mut(), counts.as_deref_mut(), offs)
            {
                tre_make_trans(left_last, right_first, trans, cnts, ofs)?;
            } else if let Some(ref mut cnts) = counts.as_deref_mut() {
                // 第一遍：仅统计
                let dummy_offs = vec![0u32; cnts.len()];
                let mut dummy_trans = Vec::new();
                tre_make_trans(left_last, right_first, &mut dummy_trans, cnts, &dummy_offs)?;
            }

            tre_ast_to_tnfa(&cat.left, tnfa, transitions.as_deref_mut(), counts.as_deref_mut(), offs)?;
            tre_ast_to_tnfa(&cat.right, tnfa, transitions.as_deref_mut(), counts.as_deref_mut(), offs)?;
        }
        AstType::Iteration => {
            let iter = match &node.obj {
                AstNodeObj::Iteration(ref i) => i,
                _ => return Err(RegError::BadPat),
            };

            // 对于无界重复（max == -1），创建从 arg.lastpos → arg.firstpos 的循环转移
            if iter.max == -1 {
                let arg_last = iter.arg.lastpos.as_deref().unwrap_or(&[]);
                let arg_first = iter.arg.firstpos.as_deref().unwrap_or(&[]);

                if let (Some(ref mut trans), Some(ref mut cnts), Some(ofs)) =
                    (transitions.as_deref_mut(), counts.as_deref_mut(), offs)
                {
                    tre_make_trans(arg_last, arg_first, trans, cnts, ofs)?;
                } else if let Some(ref mut cnts) = counts.as_deref_mut() {
                    let dummy_offs = vec![0u32; cnts.len()];
                    let mut dummy_trans = Vec::new();
                    tre_make_trans(arg_last, arg_first, &mut dummy_trans, cnts, &dummy_offs)?;
                }
            }

            tre_ast_to_tnfa(&iter.arg, tnfa, transitions.as_deref_mut(), counts.as_deref_mut(), offs)?;
        }
    }
    Ok(())
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use alloc::string::String;
    use super::*;
    use super::super::tre_mem::tre_mem_new;
    use super::super::tre::ASSERT_AT_BOL;

    // ---- tre_set_empty 测试 ----

    test!("test_set_empty_returns_empty_vec" {
        let s = tre_set_empty();
        assert!(s.is_empty());
    });

    // ---- tre_set_one 测试 ----

    test!("test_set_one_basic" {
        let mut mem = tre_mem_new();
        let set = tre_set_one(
            &mut mem,
            1,
            0x61,
            0x7a,
            None,
            0,
            None,
            None,
            None,
        );
        // 实现后：
        // assert_eq!(set.len(), 1);
        // assert_eq!(set[0].position, 1);
        // assert_eq!(set[0].code_min, 0x61);
        // assert_eq!(set[0].code_max, 0x7a);
    });

    test!("test_set_one_with_assertion" {
        let mut mem = tre_mem_new();
        let set = tre_set_one(
            &mut mem,
            2,
            0,
            0,
            None,
            ASSERT_AT_BOL,
            None,
            None,
            None,
        );
    });

    // ---- tre_set_union 测试 ----

    test!("test_set_union_two_sets" {
        let mut mem = tre_mem_new();
        let p1 = PosAndTags {
            position: 1,
            code_min: 10,
            code_max: 20,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let p2 = PosAndTags {
            position: 2,
            code_min: 30,
            code_max: 40,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let result = tre_set_union(&mut mem, &[p1], &[p2], None, 0);
        // 实现后：应返回包含两个位置的集合
    });

    test!("test_set_union_overlapping" {
        let mut mem = tre_mem_new();
        let p1 = PosAndTags {
            position: 1,
            code_min: 10,
            code_max: 20,
            tags: Some(vec![0]),
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let p2 = PosAndTags {
            position: 1, // 相同位置
            code_min: 10,
            code_max: 20,
            tags: Some(vec![1]),
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let result = tre_set_union(&mut mem, &[p1], &[p2], None, 0);
        // 相同位置应被合并
    });

    // ---- tre_match_empty 测试 ----

    test!("test_match_empty_literal_not_empty" {
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let node = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let mut tags: Vec<i32> = Vec::new();
        let mut assertions: i32 = 0;
        let mut num_tags_seen: u32 = 0;
        let result = tre_match_empty(
            &node,
            &mut tags,
            &mut assertions,
            &mut num_tags_seen,
        );
    });

    test!("test_match_empty_empty_node" {
        let lit = Literal {
            kind: LiteralKind::Empty,
            position: None,
            class: None,
            neg_classes: None,
        };
        let node = AstNode {
            node_type: AstType::Literal,
            nullable: Some(true),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let mut tags: Vec<i32> = Vec::new();
        let mut assertions: i32 = 0;
        let mut num_tags_seen: u32 = 0;
        let result = tre_match_empty(
            &node,
            &mut tags,
            &mut assertions,
            &mut num_tags_seen,
        );
    });

    // ---- tre_compute_nfl 测试 ----

    test!("test_compute_nfl_simple_literal" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let mut tree = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let result = tre_compute_nfl(&mut mem, &mut tree);
        // 实现后：
        // assert_eq!(tree.nullable, Some(false));
        // assert!(tree.firstpos.is_some());
        // assert_eq!(tree.firstpos.as_ref().unwrap().len(), 1);
    });

    test!("test_compute_nfl_catenation" {
        let mut mem = tre_mem_new();
        let lit1 = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let lit2 = Literal {
            kind: LiteralKind::Char(b'b' as i64, b'b' as i64),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let left = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit1),
        });
        let right = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit2),
        });
        let mut tree = AstNode {
            node_type: AstType::Catenation,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Catenation(Catenation { left, right }),
        };
        let result = tre_compute_nfl(&mut mem, &mut tree);
        // 实现后：nullable = false
    });

    test!("test_compute_nfl_union" {
        let mut mem = tre_mem_new();
        let lit1 = Literal {
            kind: LiteralKind::Char(b'x' as i64, b'x' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let lit2 = Literal {
            kind: LiteralKind::Char(b'y' as i64, b'y' as i64),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let left = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit1),
        });
        let right = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit2),
        });
        let mut tree = AstNode {
            node_type: AstType::Union,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Union(Union { left, right }),
        };
        let result = tre_compute_nfl(&mut mem, &mut tree);
        // 实现后：firstpos 包含 {1, 2}，lastpos 包含 {1, 2}
    });

    test!("test_compute_nfl_iteration_star" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let arg = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        });
        let mut tree = AstNode {
            node_type: AstType::Iteration,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(Iteration {
                arg,
                min: 0,
                max: -1,
                minimal: false,
            }),
        };
        let result = tre_compute_nfl(&mut mem, &mut tree);
        // 实现后：nullable = true（因为 min=0）
    });

    // ---- tre_make_trans 测试 ----

    test!("test_make_trans_two_singletons" {
        let p1 = PosAndTags {
            position: 1,
            code_min: b'a' as i64,
            code_max: b'z' as i64,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let p2 = PosAndTags {
            position: 2,
            code_min: b'0' as i64,
            code_max: b'9' as i64,
            tags: Some(vec![0, -1]),
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let mut transitions: Vec<TnfaTransition> = Vec::new();
        let mut counts: Vec<u32> = vec![0; 3];
        let mut offs: Vec<u32> = vec![0; 3];
        let result = tre_make_trans(
            &[p1],
            &[p2],
            &mut transitions,
            &mut counts,
            &mut offs,
        );
    });

    test!("test_make_trans_empty_p1" {
        let p2 = PosAndTags {
            position: 1,
            code_min: 0,
            code_max: 0,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let mut transitions: Vec<TnfaTransition> = Vec::new();
        let mut counts: Vec<u32> = vec![0; 2];
        let mut offs: Vec<u32> = vec![0; 2];
        let result = tre_make_trans(
            &[],
            &[p2],
            &mut transitions,
            &mut counts,
            &mut offs,
        );
        // 空源集合不应创建任何转移
    });

    // ---- tre_ast_to_tnfa 测试 ----

    test!("test_ast_to_tnfa_literal" {
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let node = AstNode {
            node_type: AstType::Literal,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let mut tnfa = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        let result = tre_ast_to_tnfa(&node, &mut tnfa, None, None, None);
    });

    test!("test_ast_to_tnfa_catenation" {
        let lit1 = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let lit2 = Literal {
            kind: LiteralKind::Char(b'b' as i64, b'b' as i64),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let left = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: Some(vec![PosAndTags {
                position: 1,
                code_min: b'a' as i64,
                code_max: b'a' as i64,
                tags: None,
                assertions: 0,
                class: None,
                neg_classes: None,
                backref: None,
            }]),
            lastpos: Some(vec![PosAndTags {
                position: 1,
                code_min: b'a' as i64,
                code_max: b'a' as i64,
                tags: None,
                assertions: 0,
                class: None,
                neg_classes: None,
                backref: None,
            }]),
            obj: AstNodeObj::Literal(lit1),
        });
        let right = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: Some(vec![PosAndTags {
                position: 2,
                code_min: b'b' as i64,
                code_max: b'b' as i64,
                tags: None,
                assertions: 0,
                class: None,
                neg_classes: None,
                backref: None,
            }]),
            lastpos: Some(vec![PosAndTags {
                position: 2,
                code_min: b'b' as i64,
                code_max: b'b' as i64,
                tags: None,
                assertions: 0,
                class: None,
                neg_classes: None,
                backref: None,
            }]),
            obj: AstNodeObj::Literal(lit2),
        });
        let node = AstNode {
            node_type: AstType::Catenation,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Catenation(Catenation { left, right }),
        };
        let mut tnfa = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        let result = tre_ast_to_tnfa(&node, &mut tnfa, None, None, None);
    });

    test!("test_ast_to_tnfa_iteration_star" {
        let lit = Literal {
            kind: LiteralKind::Char(b'x' as i64, b'x' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let pos = PosAndTags {
            position: 1,
            code_min: b'x' as i64,
            code_max: b'x' as i64,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        let arg = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: Some(vec![pos.clone()]),
            lastpos: Some(vec![pos]),
            obj: AstNodeObj::Literal(lit),
        });
        let node = AstNode {
            node_type: AstType::Iteration,
            nullable: Some(true),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(Iteration {
                arg,
                min: 0,
                max: -1,
                minimal: false,
            }),
        };
        let mut tnfa = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        let result = tre_ast_to_tnfa(&node, &mut tnfa, None, None, None);
    });
}
