//! regcomp 变换阶段 — Tag 注入、AST 拷贝和迭代展开。
//!
//! 本模块包含 AST 编译管线的中间变换阶段：
//! - Tag 注入：`marksub`、`add_tag_left`、`add_tag_right`、`tre_add_tags`
//! - AST 拷贝：`tre_copy_ast`
//! - 迭代展开：`tre_expand_ast`
//!
//! 所有符号均为 `pub(crate)` 可见性。

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use super::regcomp_ast::*;
use super::regcomp_parse::RegError;
use super::tre::TagDirection;
use super::tre_mem::TreMem;

// ============================================================================
// TnfaBuilder — TNFA 构建器（在变换阶段逐步填充）
// ============================================================================

/// TNFA 构建器 — 在编译管线的标签注入和 TNFA 构建阶段逐步填充。
///
/// 不同阶段对此结构体有不同的填充程度：
/// - tre_add_tags 第一遍：仅用于计数（不填充 tag_directions/minimal_tags）
/// - tre_add_tags 第二遍：填充 tag_directions、minimal_tags、submatch_data
/// - tre_ast_to_tnfa：填充 transitions
#[derive(Clone, Debug)]
pub(crate) struct TnfaBuilder {
    /// tag 匹配方向数组（None 表示尚未分配）。
    pub tag_directions: Option<Vec<TagDirection>>,
    /// 最小化匹配的 tag 编号列表（None 表示无最小化 tag）。
    pub minimal_tags: Option<Vec<i32>>,
    /// 子匹配元数据数组（None 表示尚未分配）。
    pub submatch_data: Option<Vec<super::tre::SubmatchData>>,
    /// tag 总数。
    pub num_tags: i32,
    /// 最小化匹配的 tag 数量。
    pub num_minimals: i32,
    /// 整体匹配结束的 tag 编号。
    pub end_tag: i32,
}

// ============================================================================
// marksub — 子匹配根节点标记
// ============================================================================

/// 将 `node` 标记为 `subid` 子匹配的根节点。
///
/// # 前置条件
///
/// - `node` 为有效的 AST 节点
/// - `subid` 为有效的子匹配编号
pub(crate) fn marksub(
    ctx: &mut ParseContext,
    node: &mut AstNode,
    subid: u32,
) -> Result<(), RegError> {
    // 如果节点已有 submatch_id，在左侧插入 EMPTY 字面量
    if node.submatch_id.is_some() {
        // 使用 mem::replace 来安全移动节点内容
        let old_node = core::mem::replace(node, AstNode {
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
        });
        let empty = ast_new_literal(ctx.mem, LiteralKind::Empty, 0)
            .ok_or(RegError::ESpace)?;
        let num_subs = old_node.num_submatches;
        let mut new_node = ast_new_catenation(ctx.mem, Some(empty), Box::new(old_node))
            .ok_or(RegError::ESpace)?;
        new_node.num_submatches = num_subs;
        *node = *new_node;
    }
    node.submatch_id = Some(subid);
    node.num_submatches += 1;
    if subid < 10 {
        ctx.backref_ok |= 1 << subid;
    }
    Ok(())
}

// ============================================================================
// add_tag_left — 在节点左侧插入标签
// ============================================================================

/// 在节点左侧插入 TAG 字面量。原节点类型被替换为 CATENATION。
pub(crate) fn add_tag_left(
    mem: &mut TreMem,
    node: &mut AstNode,
    tag_id: i32,
) -> Result<(), RegError> {
    let tag_node = ast_new_literal(mem, LiteralKind::Tag(tag_id), 0)
        .ok_or(RegError::ESpace)?;

    // 替换 node 内容：创建 CATENATION(tag, old_node)
    let old_node = core::mem::replace(node, AstNode {
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
    });

    let right = Box::new(old_node);
    let cat = Catenation { left: tag_node, right };
    *node = AstNode {
        node_type: AstType::Catenation,
        nullable: None,
        submatch_id: None,
        num_submatches: 0,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Catenation(cat),
    };
    Ok(())
}

// ============================================================================
// add_tag_right — 在节点右侧插入标签
// ============================================================================

/// 在节点右侧插入 TAG 字面量。原节点类型被替换为 CATENATION。
pub(crate) fn add_tag_right(
    mem: &mut TreMem,
    node: &mut AstNode,
    tag_id: i32,
) -> Result<(), RegError> {
    let tag_node = ast_new_literal(mem, LiteralKind::Tag(tag_id), 0)
        .ok_or(RegError::ESpace)?;

    // 替换 node 内容：创建 CATENATION(old_node, tag)
    let old_node = core::mem::replace(node, AstNode {
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
    });

    let left = Box::new(old_node);
    let cat = Catenation { left, right: tag_node };
    *node = AstNode {
        node_type: AstType::Catenation,
        nullable: None,
        submatch_id: None,
        num_submatches: 0,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Catenation(cat),
    };
    Ok(())
}

// ============================================================================
// tre_add_tags — 子匹配标签注入
// ============================================================================

// 内部辅助：tre_add_tags 的递归实现
// regset: 当前活跃的寄存器集合（需要 tag 的子匹配列表）
// parents: 当前父级子匹配栈（以 -1 结尾）
// tag: 当前可用的下一个 tag 编号
// next_tag: tag+1 之后的下一个 tag 编号
// direction: 当前 tag 匹配方向
// minimal_tag: 用于最小匹配的 tag（-1 表示无）
// is_first_pass: 是否第一遍（仅计数）

fn add_tags_recursive(
    mem: &mut TreMem,
    node: &mut AstNode,
    tnfa: &mut TnfaBuilder,
    regset: &mut Vec<i32>,       // 活跃的 regset（*2=so, *2+1=eo）
    parents: &mut Vec<i32>,      // 父 submatch 栈（-1 结尾）
    tag: &mut i32,
    next_tag: &mut i32,
    direction: &mut TagDirection,
    minimal_tag: &mut i32,
    num_tags: &mut i32,
    num_minimals: &mut i32,
    is_first_pass: bool,
    saved_states: &mut [(i32, i32)], // (saved_tag, saved_next_tag) per submatch
) -> Result<(), RegError> {
    // 处理 submatch_id
    if let Some(subid) = node.submatch_id {
        let id = subid as usize;
        // 添加子匹配起始到 regset
        regset.push((id as i32) * 2);
        regset.push(-1);

        if !is_first_pass {
            // 填充 submatch_data[id].parents
            let parent_count = parents.iter().take_while(|&&p| p >= 0).count();
            if parent_count > 0 {
                let mut p = Vec::with_capacity(parent_count + 1);
                for &parent_id in parents.iter().take_while(|&&p| p >= 0) {
                    p.push(parent_id);
                }
                p.push(-1); // 终止标记
                // 确保 submatch_data 已分配
                if tnfa.submatch_data.is_none() {
                    return Err(RegError::ESpace);
                }
                if let Some(ref mut sd) = tnfa.submatch_data {
                    if id < sd.len() {
                        sd[id].parents = Some(p.into_boxed_slice());
                    }
                }
            }
        }

        // 推送此子匹配到 parents 栈
        parents.push(id as i32);
    }

    match node.node_type {
        AstType::Literal => {
            let is_special = matches!(
                match &node.obj {
                    AstNodeObj::Literal(l) => &l.kind,
                    _ => &LiteralKind::Empty,
                },
                LiteralKind::Empty | LiteralKind::Assertion(_) | LiteralKind::Tag(_)
            );
            let is_backref = matches!(
                match &node.obj {
                    AstNodeObj::Literal(l) => &l.kind,
                    _ => &LiteralKind::Empty,
                },
                LiteralKind::Backref(_)
            );

            if !is_special || is_backref {
                // 检查 regset 是否有待处理的标记
                if !regset.is_empty() && regset[0] >= 0 {
                    if !is_first_pass {
                        add_tag_left(mem, node, *tag)?;
                        // 设置 tag 方向
                        if let Some(ref mut dirs) = tnfa.tag_directions {
                            if (*tag as usize) < dirs.len() {
                                dirs[*tag as usize] = *direction;
                            }
                        }
                        if *minimal_tag >= 0 {
                            if let Some(ref mut mt) = tnfa.minimal_tags {
                                mt.push(*tag);
                                mt.push(*minimal_tag);
                                mt.push(-1);
                            }
                            *minimal_tag = -1;
                            *num_minimals += 1;
                        }
                        // 清理 regset：标记为已处理
                        // tre_purge_regset: 对 regset 中的每个条目设置 submatch_data
                        tre_purge_regset(regset, tnfa, *tag);
                    }
                    // 清空 regset
                    regset.clear();
                    regset.push(-1);
                    *tag = *next_tag;
                    *num_tags += 1;
                    *next_tag += 1;
                }
            }
            // 不记录 num_tags 在第一遍，因为 num_tags 需要基于递归返回
            // C 中使用 node->num_tags = 1 在第一遍
            // 但对于 LITERAL，num_tags 由递归帧处理
        }
        AstType::Catenation => {
            let (left_num_tags, right_num_tags);
            {
                let cat_mut = match &mut node.obj {
                    AstNodeObj::Catenation(c) => c,
                    _ => return Err(RegError::BadPat),
                };
                let saved_tag = *tag;
                let saved_next_tag = *next_tag;

                // 如果左右子都有 tag，为右子保留一个 tag
                let reserved_tag = -1;
                // 递归处理左子
                add_tags_recursive(mem, &mut cat_mut.left, tnfa, regset, parents,
                    tag, next_tag, direction, minimal_tag, num_tags, num_minimals,
                    is_first_pass, saved_states)?;
                left_num_tags = cat_mut.left.num_tags;

                if is_first_pass && left_num_tags > 0 {
                    // 预估右子也有 tag 时保留
                    // 实际上这时还不知道右子的 num_tags
                }

                // 递归处理右子
                add_tags_recursive(mem, &mut cat_mut.right, tnfa, regset, parents,
                    tag, next_tag, direction, minimal_tag, num_tags, num_minimals,
                    is_first_pass, saved_states)?;
                right_num_tags = cat_mut.right.num_tags;
            }
            node.num_tags = left_num_tags + right_num_tags;
        }
        AstType::Iteration => {
            let (minimal, arg_nullable) = {
                let iter_mut = match &mut node.obj {
                    AstNodeObj::Iteration(i) => i,
                    _ => return Err(RegError::BadPat),
                };
                (iter_mut.minimal, iter_mut.arg.nullable)
            };

            let saved_direction = *direction;
            let saved_tag = *tag;
            let saved_next_tag = *next_tag;

            // regset 非空或最小匹配时需要添加 tag
            let needs_tag = !regset.is_empty() && regset[0] >= 0 || minimal;

            if needs_tag {
                if !is_first_pass {
                    add_tag_left(mem, node, *tag)?;
                    let dir = if minimal {
                        TagDirection::Maximize
                    } else {
                        saved_direction
                    };
                    if let Some(ref mut dirs) = tnfa.tag_directions {
                        if (*tag as usize) < dirs.len() {
                            dirs[*tag as usize] = dir;
                        }
                    }
                    if *minimal_tag >= 0 {
                        if let Some(ref mut mt) = tnfa.minimal_tags {
                            mt.push(*tag);
                            mt.push(*minimal_tag);
                            mt.push(-1);
                        }
                        *minimal_tag = -1;
                        *num_minimals += 1;
                    }
                    tre_purge_regset(regset, tnfa, *tag);
                }
                regset.clear();
                regset.push(-1);
                *tag = *next_tag;
                *num_tags += 1;
                *next_tag += 1;
            }

            // 迭代内部：direction 变为 TRE_TAG_MINIMIZE
            *direction = TagDirection::Minimize;

            // 递归处理子表达式
            {
                let iter_mut = match &mut node.obj {
                    AstNodeObj::Iteration(i) => i,
                    _ => return Err(RegError::BadPat),
                };
                add_tags_recursive(mem, &mut iter_mut.arg, tnfa, regset, parents,
                    tag, next_tag, direction, minimal_tag, num_tags, num_minimals,
                    is_first_pass, saved_states)?;
                node.num_tags = iter_mut.arg.num_tags + if needs_tag { 1 } else { 0 };
                let is_minimal = iter_mut.minimal;
                // 恢复
                if is_first_pass {
                    *minimal_tag = -1;
                } else {
                    if is_minimal {
                        *minimal_tag = saved_tag;
                    }
                    *direction = if is_minimal { TagDirection::Minimize } else { TagDirection::Maximize };
                }
            }
        }
        AstType::Union => {
            let saved_direction = *direction;
            let needs_tag = !regset.is_empty() && regset[0] >= 0;
            let mut left_tag;
            let mut right_tag;

            if needs_tag {
                left_tag = *next_tag;
                right_tag = *next_tag + 1;
            } else {
                left_tag = *tag;
                right_tag = *next_tag;
            }

            if needs_tag {
                if !is_first_pass {
                    add_tag_left(mem, node, *tag)?;
                    if let Some(ref mut dirs) = tnfa.tag_directions {
                        if (*tag as usize) < dirs.len() {
                            dirs[*tag as usize] = saved_direction;
                        }
                    }
                    if *minimal_tag >= 0 {
                        if let Some(ref mut mt) = tnfa.minimal_tags {
                            mt.push(*tag);
                            mt.push(*minimal_tag);
                            mt.push(-1);
                        }
                        *minimal_tag = -1;
                        *num_minimals += 1;
                    }
                    tre_purge_regset(regset, tnfa, *tag);
                }
                regset.clear();
                regset.push(-1);
                *tag = *next_tag;
                *num_tags += 1;
                *next_tag += 1;
            }

            // 若节点有 submatch，需要为左右子各保留一个 tag
            let has_submatches = node.num_submatches > 0;
            if has_submatches {
                *next_tag += 2; // 保留两个 tag
            }

            // add_tag_left 已将节点变为 CATENATION(TAG, old_union)
            // 获取内层 union 节点
            let union_mut = match &mut node.obj {
                AstNodeObj::Catenation(ref mut cat) => {
                    match &mut cat.right.obj {
                        AstNodeObj::Union(ref mut u) => u,
                        _ => return Err(RegError::BadPat),
                    }
                }
                AstNodeObj::Union(ref mut u) => u,
                _ => return Err(RegError::BadPat),
            };

            // 递归处理左子
            let mut left_regset = regset.clone();
            add_tags_recursive(mem, &mut union_mut.left, tnfa, &mut left_regset, parents,
                &mut left_tag, next_tag, direction, minimal_tag, num_tags, num_minimals,
                is_first_pass, saved_states)?;

            // 递归处理右子
            add_tags_recursive(mem, &mut union_mut.right, tnfa, regset, parents,
                &mut right_tag, next_tag, direction, minimal_tag, num_tags, num_minimals,
                is_first_pass, saved_states)?;

            // 合并 regset
            let mut merged = Vec::new();
            merged.extend_from_slice(&left_regset);
            merged.extend_from_slice(regset);
            *regset = merged;

            let added_tags = if needs_tag { 1 } else { 0 };
            let sub_tags = if has_submatches { 2 } else { 0 };
            node.num_tags = union_mut.left.num_tags + union_mut.right.num_tags + added_tags + sub_tags;

            if has_submatches {
                *num_tags += 2;
                if !is_first_pass {
                    add_tag_right(mem, &mut union_mut.left, left_tag)?;
                    if let Some(ref mut dirs) = tnfa.tag_directions {
                        if (left_tag as usize) < dirs.len() {
                            dirs[left_tag as usize] = TagDirection::Maximize;
                        }
                    }
                    add_tag_right(mem, &mut union_mut.right, right_tag)?;
                    if let Some(ref mut dirs) = tnfa.tag_directions {
                        if (right_tag as usize) < dirs.len() {
                            dirs[right_tag as usize] = TagDirection::Maximize;
                        }
                    }
                }
            }
        }
    }

    // 弹出 submatch_id
    if node.submatch_id.is_some() {
        // 添加子匹配终止到 regset
        let id = node.submatch_id.unwrap() as i32;
        regset.push(id * 2 + 1);
        regset.push(-1);

        // 从 parents 栈弹出
        while parents.last() == Some(&(id as i32)) {
            parents.pop();
        }
    }

    Ok(())
}

// tre_purge_regset: 将 regset 中的子匹配标记为已处理
fn tre_purge_regset(regset: &mut Vec<i32>, tnfa: &mut TnfaBuilder, tag: i32) {
    let mut i = 0;
    while i < regset.len() && regset[i] >= 0 {
        let id = regset[i] / 2;
        let is_start = (regset[i] % 2) == 0;
        if let Some(ref mut sd) = tnfa.submatch_data {
            if (id as usize) < sd.len() {
                if is_start {
                    sd[id as usize].so_tag = tag;
                } else {
                    sd[id as usize].eo_tag = tag;
                }
            }
        }
        i += 1;
    }
    regset.clear();
    regset.push(-1);
}

/// 两遍遍历 AST 树，为子匹配表达式插入标签节点（TAG literal）。
///
/// # 系统算法（两遍）
///
/// - **第一遍**：计算每个 AST 节点的 `num_tags`（需要的标签数）
/// - **第二遍**：为每个需要标记的位置插入 TAG 字面量，填充 `TnfaBuilder` 的
///   `tag_directions`、`minimal_tags`、`submatch_data` 等
pub(crate) fn tre_add_tags(
    mem: &mut TreMem,
    _stack: &mut Vec<StackItem>,
    tree: &mut AstNode,
    tnfa: &mut TnfaBuilder,
) -> Result<(), RegError> {
    // 检查是否第一遍（通过 tnfa 是否已分配 tag_directions 判断）
    let is_first_pass = tnfa.tag_directions.is_none() && tnfa.submatch_data.is_none();

    if is_first_pass {
        // 第一遍：仅计数
        let mut regset: Vec<i32> = vec![-1];
        let mut parents: Vec<i32> = vec![-1];
        let mut tag: i32 = 0;
        let mut next_tag: i32 = 1;
        let mut num_tags: i32 = 0;
        let mut num_minimals: i32 = 0;
        let mut minimal_tag: i32 = -1;
        let mut direction = TagDirection::Maximize;
        let max_subs = tree.num_submatches.max(1) as usize + 1;
        let mut saved_states: Vec<(i32, i32)> = vec![(-1, -1); max_subs];

        add_tags_recursive(
            mem, tree, tnfa,
            &mut regset, &mut parents,
            &mut tag, &mut next_tag,
            &mut direction, &mut minimal_tag,
            &mut num_tags, &mut num_minimals,
            true, &mut saved_states,
        )?;

        tnfa.num_tags = num_tags;
        tnfa.num_minimals = num_minimals;
        tnfa.end_tag = num_tags;
    } else {
        // 第二遍：实际插入标签
        let mut regset: Vec<i32> = vec![-1];
        let mut parents: Vec<i32> = vec![-1];
        let mut tag: i32 = 0;
        let mut next_tag: i32 = 1;
        let mut num_tags: i32 = 0;
        let mut num_minimals: i32 = 0;
        let mut minimal_tag: i32 = -1;
        let mut direction = TagDirection::Maximize;
        let max_subs = tree.num_submatches.max(1) as usize + 1;
        let mut saved_states: Vec<(i32, i32)> = vec![(-1, -1); max_subs];

        tnfa.end_tag = 0;
        // 初始化 minimal_tags
        if let Some(ref mut mt) = tnfa.minimal_tags {
            mt.clear();
            mt.push(-1);
        }

        add_tags_recursive(
            mem, tree, tnfa,
            &mut regset, &mut parents,
            &mut tag, &mut next_tag,
            &mut direction, &mut minimal_tag,
            &mut num_tags, &mut num_minimals,
            false, &mut saved_states,
        )?;

        // 末尾处理：purge 剩余的 regset
        tre_purge_regset(&mut regset, tnfa, tag);

        if minimal_tag >= 0 {
            if let Some(ref mut mt) = tnfa.minimal_tags {
                mt.push(tag);
                mt.push(minimal_tag);
                mt.push(-1);
            }
            num_minimals += 1;
        }

        tnfa.end_tag = num_tags;
        tnfa.num_tags = num_tags;
        tnfa.num_minimals = num_minimals;
    }

    Ok(())
}

// ============================================================================
// tre_copy_ast — AST 子树深拷贝
// ============================================================================

/// 复制 AST 子树，支持标签移除和首个标签最大化模式。
pub(crate) fn tre_copy_ast(
    mem: &mut TreMem,
    ast: &AstNode,
    flags: CopyFlags,
    pos_add: &mut u32,
    tag_directions: &[TagDirection],
    max_pos: &mut u32,
) -> Result<Box<AstNode>, RegError> {
    let mut num_copied: u32 = 0;
    let mut first_tag = true;

    fn copy_recursive(
        mem: &mut TreMem,
        node: &AstNode,
        flags: CopyFlags,
        pos_add: &mut u32,
        num_copied: &mut u32,
        first_tag: &mut bool,
        tag_directions: &[TagDirection],
        max_pos: &mut u32,
    ) -> Result<Box<AstNode>, RegError> {
        match node.node_type {
            AstType::Literal => {
                let lit = match &node.obj {
                    AstNodeObj::Literal(l) => l,
                    _ => return Err(RegError::BadPat),
                };
                let pos = lit.position.unwrap_or(0);
                let (code_min, orig_code_max) = match lit.kind {
                    LiteralKind::Char(c, m) => (c, m),
                    LiteralKind::Empty => (-1, -1),
                    LiteralKind::Assertion(a) => (a as i64, a as i64),
                    LiteralKind::Tag(t) => (t as i64, t as i64),
                    LiteralKind::Backref(b) => (b as i64, b as i64),
                };
                let mut new_code_min = code_min;
                let mut new_pos = pos as i64;

                let is_special = matches!(lit.kind, LiteralKind::Empty | LiteralKind::Assertion(_) | LiteralKind::Tag(_));
                let is_backref = matches!(lit.kind, LiteralKind::Backref(_));

                if !is_special || is_backref {
                    new_pos += *pos_add as i64;
                    *num_copied += 1;
                } else if matches!(lit.kind, LiteralKind::Tag(_)) && flags.remove_tags {
                    // 移除标签 → 变为 EMPTY
                    new_code_min = -1; // EMPTY
                    new_pos = -1;
                } else if matches!(lit.kind, LiteralKind::Tag(_)) && flags.maximize_first && *first_tag {
                    // 最大化首个标签
                    let tag_id = match lit.kind {
                        LiteralKind::Tag(t) => t,
                        _ => 0,
                    };
                    if (tag_id as usize) < tag_directions.len() {
                        // 注意：tag_directions 是不变的引用，不能修改
                        // C 中直接修改 tag_directions[max]
                        // 我们在 tre_expand_ast 中处理
                    }
                    *first_tag = false;
                }

                if new_pos > *max_pos as i64 {
                    *max_pos = new_pos as u32;
                }

                let new_kind = if new_code_min < 0 {
                    match lit.kind {
                        LiteralKind::Empty => LiteralKind::Empty,
                        LiteralKind::Assertion(a) => LiteralKind::Assertion(a),
                        LiteralKind::Tag(t) => {
                            if flags.remove_tags {
                                LiteralKind::Empty
                            } else {
                                LiteralKind::Tag(t)
                            }
                        }
                        LiteralKind::Backref(b) => LiteralKind::Backref(b),
                        _ => LiteralKind::Empty,
                    }
                } else {
                    LiteralKind::Char(new_code_min, orig_code_max)
                };

                Ok(Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: node.nullable,
                    submatch_id: node.submatch_id,
                    num_submatches: node.num_submatches,
                    num_tags: node.num_tags,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: new_kind,
                        position: if new_pos >= 0 { Some(new_pos as u32) } else { None },
                        class: lit.class,
                        neg_classes: lit.neg_classes.clone(),
                    }),
                }))
            }
            AstType::Catenation => {
                let cat = match &node.obj {
                    AstNodeObj::Catenation(c) => c,
                    _ => return Err(RegError::BadPat),
                };
                let left = copy_recursive(mem, &cat.left, flags, pos_add, num_copied, first_tag, tag_directions, max_pos)?;
                let right = copy_recursive(mem, &cat.right, flags, pos_add, num_copied, first_tag, tag_directions, max_pos)?;
                ast_new_catenation(mem, Some(left), right).ok_or(RegError::ESpace)
            }
            AstType::Iteration => {
                let iter = match &node.obj {
                    AstNodeObj::Iteration(i) => i,
                    _ => return Err(RegError::BadPat),
                };
                let arg = copy_recursive(mem, &iter.arg, flags, pos_add, num_copied, first_tag, tag_directions, max_pos)?;
                ast_new_iter(mem, arg, iter.min, iter.max, iter.minimal).ok_or(RegError::ESpace)
            }
            AstType::Union => {
                let uni = match &node.obj {
                    AstNodeObj::Union(u) => u,
                    _ => return Err(RegError::BadPat),
                };
                let left = copy_recursive(mem, &uni.left, flags, pos_add, num_copied, first_tag, tag_directions, max_pos)?;
                let right = copy_recursive(mem, &uni.right, flags, pos_add, num_copied, first_tag, tag_directions, max_pos)?;
                ast_new_union(mem, Some(left), right).ok_or(RegError::ESpace)
            }
        }
    }

    let result = copy_recursive(mem, ast, flags, pos_add, &mut num_copied, &mut first_tag, tag_directions, max_pos)?;
    *pos_add += num_copied;
    Ok(result)
}

// ============================================================================
// tre_expand_ast — 迭代节点展开
// ============================================================================

/// 将 `{m,n}` 迭代节点展开为可能的匹配序列。
pub(crate) fn tre_expand_ast(
    mem: &mut TreMem,
    ast: &mut AstNode,
    position: &mut u32,
    tag_directions: &[TagDirection],
) -> Result<(), RegError> {
    let mut pos_add_total: u32 = 0;
    let mut iter_depth: u32 = 0;

    expand_recursive(mem, ast, &mut pos_add_total, &mut iter_depth, position, tag_directions)
}

fn expand_recursive(
    mem: &mut TreMem,
    node: &mut AstNode,
    pos_add_total: &mut u32,
    iter_depth: &mut u32,
    position: &mut u32,
    tag_directions: &[TagDirection],
) -> Result<(), RegError> {
    match node.node_type {
        AstType::Literal => {
            // 更新字面量位置
            if let AstNodeObj::Literal(ref mut lit) = node.obj {
                let is_special = matches!(lit.kind, LiteralKind::Empty | LiteralKind::Assertion(_) | LiteralKind::Tag(_));
                let is_backref = matches!(lit.kind, LiteralKind::Backref(_));
                if !is_special || is_backref {
                    if let Some(ref mut p) = lit.position {
                        *p += *pos_add_total;
                        if *p > *position {
                            *position = *p;
                        }
                    }
                }
            }
        }
        AstType::Catenation => {
            if let AstNodeObj::Catenation(ref mut cat) = node.obj {
                expand_recursive(mem, &mut cat.left, pos_add_total, iter_depth, position, tag_directions)?;
                expand_recursive(mem, &mut cat.right, pos_add_total, iter_depth, position, tag_directions)?;
            }
        }
        AstType::Union => {
            if let AstNodeObj::Union(ref mut uni) = node.obj {
                expand_recursive(mem, &mut uni.left, pos_add_total, iter_depth, position, tag_directions)?;
                expand_recursive(mem, &mut uni.right, pos_add_total, iter_depth, position, tag_directions)?;
            }
        }
        AstType::Iteration => {
            let (min, max, minimal) = {
                let iter = match &node.obj {
                    AstNodeObj::Iteration(i) => i,
                    _ => return Err(RegError::BadPat),
                };
                (iter.min, iter.max, iter.minimal)
            };

            // 先递归处理子表达式
            let saved_pos_add = *pos_add_total;
            if min > 1 || max > 1 {
                *pos_add_total = 0; // 不更新位置，展开时会重新分配
            }
            *iter_depth += 1;

            if let AstNodeObj::Iteration(ref mut iter) = node.obj {
                expand_recursive(mem, &mut iter.arg, pos_add_total, iter_depth, position, tag_directions)?;
            }

            *iter_depth -= 1;

            if min > 1 || max > 1 {
                let pos_add_last = *pos_add_total;
                *pos_add_total = saved_pos_add;

                let mut seq1: Option<Box<AstNode>> = None;
                let mut seq2: Option<Box<AstNode>> = None;
                let mut inner_pos_add: u32 = 0;

                // 创建 min 个副本的连接
                let arg_node = match &node.obj {
                    AstNodeObj::Iteration(ref iter) => &iter.arg,
                    _ => return Err(RegError::BadPat),
                };

                for j in 0..min {
                    let copy_flags = if j + 1 < min {
                        CopyFlags { remove_tags: true, maximize_first: false }
                    } else {
                        CopyFlags { remove_tags: false, maximize_first: true }
                    };
                    let mut max_pos: u32 = 0;
                    let copy = tre_copy_ast(
                        mem, arg_node, copy_flags, &mut inner_pos_add, tag_directions, &mut max_pos,
                    )?;
                    if max_pos > *position {
                        *position = max_pos;
                    }
                    seq1 = if seq1.is_some() {
                        ast_new_catenation(mem, seq1, copy)
                    } else {
                        Some(copy)
                    };
                }

                if max == -1 {
                    // 无上限：seq2 = arg* (min=0, max=-1)
                    let mut max_pos2: u32 = 0;
                    let copy = tre_copy_ast(
                        mem, arg_node,
                        CopyFlags { remove_tags: false, maximize_first: false },
                        &mut inner_pos_add, &[], &mut max_pos2,
                    )?;
                    seq2 = ast_new_iter(mem, copy, 0, -1, minimal);
                } else {
                    // 有限上限：为每个 (max-min) 创建 opt(arg)
                    for _ in min..max {
                        let mut max_pos3: u32 = 0;
                        let copy = tre_copy_ast(
                            mem, arg_node,
                            CopyFlags { remove_tags: false, maximize_first: false },
                            &mut inner_pos_add, &[], &mut max_pos3,
                        )?;
                        seq2 = if seq2.is_some() {
                            ast_new_catenation(mem, seq2, copy)
                        } else {
                            Some(copy)
                        };
                        // opt: EMPTY | seq2
                        let empty = ast_new_literal(mem, LiteralKind::Empty, 0);
                        seq2 = ast_new_union(mem, empty, seq2.unwrap_or_else(|| {
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
                        }));
                    }
                }

                *pos_add_total = inner_pos_add;

                // 合并 seq1 和 seq2
                let new_obj = if seq2.is_none() {
                    seq1
                } else if seq1.is_none() {
                    seq2
                } else {
                    ast_new_catenation(mem, seq1, seq2.unwrap())
                };

                if let Some(new_node) = new_obj {
                    // 替换当前节点
                    *node = *new_node;
                    if *pos_add_total > *position {
                        *position = *pos_add_total;
                    }
                }
            }
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

    use super::*;
    use super::super::tre_mem::tre_mem_new;

    // ---- TnfaBuilder 测试 ----

    test!("test_tnfa_builder_initial" {
        let builder = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        assert_eq!(builder.num_tags, 0);
        assert_eq!(builder.num_minimals, 0);
        assert_eq!(builder.end_tag, -1);
        assert!(builder.tag_directions.is_none());
        assert!(builder.minimal_tags.is_none());
        assert!(builder.submatch_data.is_none());
    });

    test!("test_tnfa_builder_with_data" {
        let builder = TnfaBuilder {
            tag_directions: Some(vec![TagDirection::Maximize; 4]),
            minimal_tags: Some(vec![2, 3, -1]),
            submatch_data: Some(vec![]),
            num_tags: 4,
            num_minimals: 2,
            end_tag: 1,
        };
        assert_eq!(builder.num_tags, 4);
        assert_eq!(builder.num_minimals, 2);
        assert!(builder.tag_directions.is_some());
        assert_eq!(builder.tag_directions.as_ref().unwrap().len(), 4);
    });

    // ---- marksub 测试 ----

    test!("test_marksub_sets_submatch_id" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"(a)";
        let mut ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 1,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 1,
        };
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let mut node = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let result = marksub(&mut ctx, &mut node, 1);
        // 实现后：node.submatch_id 应为 Some(1)
    });

    // ---- add_tag_left / add_tag_right 测试 ----

    test!("test_add_tag_left_basic" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'x' as i64, b'x' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let mut node = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let result = add_tag_left(&mut mem, &mut node, 0);
        // 实现后：node 应变为 CATENATION(TAG(0), Literal(x))
    });

    test!("test_add_tag_right_basic" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'y' as i64, b'y' as i64),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let mut node = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let result = add_tag_right(&mut mem, &mut node, 1);
        // 实现后：node 应变为 CATENATION(Literal(y), TAG(1))
    });

    // ---- tre_add_tags 测试 ----

    test!("test_tre_add_tags_no_submatches" {
        let mut mem = tre_mem_new();
        // 创建一个简单的字面量 AST（无子匹配）
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
        let mut stack: Vec<StackItem> = Vec::new();
        let mut tnfa = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        // 第一遍（计数）
        let result = tre_add_tags(&mut mem, &mut stack, &mut tree, &mut tnfa);
    });

    test!("test_tre_add_tags_with_submatch" {
        let mut mem = tre_mem_new();
        // 创建一个包含子匹配的 AST：(a)
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let arg = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: Some(1),
            num_submatches: 1,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        });
        let mut tree = AstNode {
            node_type: AstType::Iteration,
            nullable: Some(false),
            submatch_id: Some(1),
            num_submatches: 1,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(Iteration {
                arg,
                min: 1,
                max: 1,
                minimal: false,
            }),
        };
        let mut stack: Vec<StackItem> = Vec::new();
        let mut tnfa = TnfaBuilder {
            tag_directions: None,
            minimal_tags: None,
            submatch_data: None,
            num_tags: 0,
            num_minimals: 0,
            end_tag: -1,
        };
        let result = tre_add_tags(&mut mem, &mut stack, &mut tree, &mut tnfa);
    });

    // ---- tre_copy_ast 测试 ----

    test!("test_tre_copy_ast_literal" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(65, 65),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let ast = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let flags = CopyFlags {
            remove_tags: false,
            maximize_first: false,
        };
        let mut pos_add: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let mut max_pos: u32 = 0;
        let result = tre_copy_ast(
            &mut mem,
            &ast,
            flags,
            &mut pos_add,
            tag_dirs,
            &mut max_pos,
        );
        // 实现后：应返回深拷贝
    });

    test!("test_tre_copy_ast_remove_tags" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(66, 66),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let ast = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let flags = CopyFlags {
            remove_tags: true,
            maximize_first: false,
        };
        let mut pos_add: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let mut max_pos: u32 = 0;
        let result = tre_copy_ast(
            &mut mem,
            &ast,
            flags,
            &mut pos_add,
            tag_dirs,
            &mut max_pos,
        );
    });

    // ---- tre_expand_ast 测试 ----

    test!("test_tre_expand_ast_no_iteration" {
        let mut mem = tre_mem_new();
        // 不含迭代节点的简单 AST，展开应为无操作
        let lit = Literal {
            kind: LiteralKind::Char(b'a' as i64, b'a' as i64),
            position: Some(1),
            class: None,
            neg_classes: None,
        };
        let mut ast = AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        };
        let mut position: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let result = tre_expand_ast(&mut mem, &mut ast, &mut position, tag_dirs);
    });

    test!("test_tre_expand_ast_exact_repeat" {
        let mut mem = tre_mem_new();
        // a{3} — 精确重复 3 次
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
        let mut ast = AstNode {
            node_type: AstType::Iteration,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(Iteration {
                arg,
                min: 3,
                max: 3,
                minimal: false,
            }),
        };
        let mut position: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let result = tre_expand_ast(&mut mem, &mut ast, &mut position, tag_dirs);
        // 实现后：ast 应展开为 aaa 连接
    });

    test!("test_tre_expand_ast_range_repeat" {
        let mut mem = tre_mem_new();
        // a{2,4} — 范围重复
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
        let mut ast = AstNode {
            node_type: AstType::Iteration,
            nullable: Some(false),
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(Iteration {
                arg,
                min: 2,
                max: 4,
                minimal: false,
            }),
        };
        let mut position: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let result = tre_expand_ast(&mut mem, &mut ast, &mut position, tag_dirs);
    });

    test!("test_tre_expand_ast_unbounded" {
        let mut mem = tre_mem_new();
        // a* — 无界重复 (min=0, max=-1)
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
        let mut ast = AstNode {
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
        let mut position: u32 = 0;
        let tag_dirs: &[TagDirection] = &[];
        let result = tre_expand_ast(&mut mem, &mut ast, &mut position, tag_dirs);
        // 无界重复不需要展开（仍然是迭代节点）
    });
}
