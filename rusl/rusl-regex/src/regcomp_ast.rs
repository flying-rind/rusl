//! regcomp AST 类型定义 — 正则表达式抽象语法树的数据结构和构造函数。
//!
//! 本模块为 regcomp 的子模块，包含 AST 节点类型、字面量表示、
//! 解析上下文以及 AST 节点的构造函数。所有符号均为 `pub(crate)` 可见性。
//!
//! # 对应关系
//!
//! 对应 regcomp.md spec 的第一至第三部分（AST 类型定义和构造函数）。

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::c_int;

use super::tre::{TreCint, TreCtype};
use super::tre_mem::TreMem;

// ============================================================================
// AstType — AST 节点类型枚举
// ============================================================================

/// AST 节点类型。
///
/// 对应 C 的 `LITERAL`、`CATENATION`、`ITERATION`、`UNION` 分类。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AstType {
    /// 字面量节点（单字符、断言、标签、反向引用、空串）。
    Literal,
    /// 连接节点（串接）。
    Catenation,
    /// 迭代节点（`*` `+` `?` `{m,n}`）。
    Iteration,
    /// 并集节点（`|`）。
    Union,
}

// ============================================================================
// LiteralKind — 字面量子类型
// ============================================================================

/// 字面量节点的子类型。
///
/// C 实现通过 `code_min` 的负值编码区分特殊节点类型（EMPTY=-1、
/// ASSERTION=-2、TAG=-3、BACKREF=-4）。Rust 使用带数据的枚举替代，
/// 类型安全且无需 IS_SPECIAL / IS_EMPTY 等宏。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LiteralKind {
    /// 普通字符（`(code_min, code_max)` 范围，包括 Unicode 码点）。
    Char(i64, i64),
    /// 空叶节点（匹配空串）。
    Empty,
    /// 断言叶节点（携带断言类型位掩码，见 ASSERT_* 常量）。
    Assertion(i32),
    /// 标签叶节点（携带 tag_id）。
    Tag(i32),
    /// 反向引用叶节点（携带引用编号）。
    Backref(i32),
}

// ============================================================================
// AstNodeObj — AST 节点数据载体
// ============================================================================

/// AST 节点数据 — 根据 `node_type` 的不同携带不同类型的数据。
#[derive(Clone, Debug)]
pub(crate) enum AstNodeObj {
    /// 字面量节点数据。
    Literal(Literal),
    /// 连接节点数据。
    Catenation(Catenation),
    /// 迭代节点数据。
    Iteration(Iteration),
    /// 并集节点数据。
    Union(Union),
}

// ============================================================================
// AstNode — AST 通用节点
// ============================================================================

/// AST 通用节点。
///
/// 表示正则表达式 AST 树中的一个节点。包含类型标签、计算的 NFL 属性、
/// 子匹配标识，以及类型特定的子数据。
#[derive(Clone, Debug)]
pub(crate) struct AstNode {
    /// 节点类型。
    pub node_type: AstType,
    /// 可空性：`None` = 未计算，`Some(false)` = 不可空，`Some(true)` = 可空。
    pub nullable: Option<bool>,
    /// 子匹配根节点 ID（None 表示非子匹配根）。
    pub submatch_id: Option<u32>,
    /// 子树中的子匹配数量。
    pub num_submatches: u32,
    /// 子树中的 tag 数量。
    pub num_tags: u32,
    /// firstpos 集合：`None` = 未计算。
    pub firstpos: Option<Vec<PosAndTags>>,
    /// lastpos 集合：`None` = 未计算。
    pub lastpos: Option<Vec<PosAndTags>>,
    /// 类型特定的节点数据。
    pub obj: AstNodeObj,
}

// ============================================================================
// Literal — 字面量节点数据
// ============================================================================

/// 字面量节点数据。
///
/// 替代 C 的 `code_min`/`code_max` 双字段 + 负值编码方案。
#[derive(Clone, Debug)]
pub(crate) struct Literal {
    /// 字面量种类（替代 C 的 `code_min < 0` 负值编码）。
    pub kind: LiteralKind,
    /// 在正则表达式中的位置序号。
    pub position: Option<u32>,
    /// 正向字符类别（如 `[:alnum:]`）。
    pub class: Option<TreCtype>,
    /// 否定字符类别列表（以 0 结尾的 TreCtype 数组）。
    pub neg_classes: Option<Vec<TreCtype>>,
}

// ============================================================================
// Catenation — 连接节点数据
// ============================================================================

/// 连接节点数据。
#[derive(Clone, Debug)]
pub(crate) struct Catenation {
    /// 左子表达式（除最后一个外的所有）。
    pub left: Box<AstNode>,
    /// 右子表达式（最后一个）。
    pub right: Box<AstNode>,
}

// ============================================================================
// Iteration — 迭代节点数据
// ============================================================================

/// 迭代节点数据。
#[derive(Clone, Debug)]
pub(crate) struct Iteration {
    /// 被迭代的子表达式。
    pub arg: Box<AstNode>,
    /// 最小重复次数。
    pub min: i32,
    /// 最大重复次数（-1 表示无上限）。
    pub max: i32,
    /// `true` = 非贪婪匹配（`*?`、`+?`）。
    pub minimal: bool,
}

// ============================================================================
// Union — 并集节点数据
// ============================================================================

/// 并集节点数据。
#[derive(Clone, Debug)]
pub(crate) struct Union {
    /// 左分支。
    pub left: Box<AstNode>,
    /// 右分支。
    pub right: Box<AstNode>,
}

// ============================================================================
// PosAndTags — 位置-标签组合
// ============================================================================

/// AST 位置中的位置-标签组合。
///
/// 用于 NFL 计算中表示某个特定字符位置及其关联的标签集合。
#[derive(Clone, Debug)]
pub(crate) struct PosAndTags {
    /// 位置序号。
    pub position: i32,
    /// 字符范围下限。
    pub code_min: i64,
    /// 字符范围上限。
    pub code_max: i64,
    /// 关联的 tag 编号列表（None = 空）。
    pub tags: Option<Vec<i32>>,
    /// 断言位掩码。
    pub assertions: i32,
    /// 正向字符类别（None = 无）。
    pub class: Option<TreCtype>,
    /// 否定字符类别列表（None = 无）。
    pub neg_classes: Option<Vec<TreCtype>>,
    /// 反向引用编号（None = 非反向引用）。
    pub backref: Option<i32>,
}

// ============================================================================
// LiteralsBuilder — 字面量数组构造器
// ============================================================================

/// 字面量数组构造器。
///
/// C 实现手动管理动态数组（`tre_literal_t **a` + `len` + `cap` + `realloc`）。
/// Rust 使用 `Vec<Literal>` 自动管理容量和增长。
#[derive(Clone, Debug)]
pub(crate) struct LiteralsBuilder {
    /// 字面量列表。
    pub literals: Vec<Literal>,
}

// ============================================================================
// NegCollector — 否定字符类收集器
// ============================================================================

/// 否定字符类收集器。
///
/// 用于解析 `[^...]` 括号表达式时收集否定字符类。
/// C 实现使用固定 64 元素数组 + `len` 字段；
/// Rust 使用 `Vec<TreCtype>` 自动增长。
#[derive(Clone, Debug)]
pub(crate) struct NegCollector {
    /// 是否取反。
    pub negate: bool,
    /// 否定字符类列表（以 0 结尾）。
    pub classes: Vec<TreCtype>,
}

// ============================================================================
// StackItem — 解析栈元素
// ============================================================================

/// 显式解析栈的元素。
///
/// 用于 `tre_parse` 等迭代式遍历函数中维护解析状态。
#[derive(Clone, Debug)]
pub(crate) struct StackItem {
    /// 当前子表达式 ID。
    pub subid: u32,
    /// 当前分支的累积 AST 节点。
    pub node: Option<Box<AstNode>>,
}

// ============================================================================
// CopyFlags — AST 拷贝标志
// ============================================================================

/// `tre_copy_ast` 使用的拷贝控制标志。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CopyFlags {
    /// 拷贝时移除标签节点。
    pub remove_tags: bool,
    /// 使用首个标签最大化模式。
    pub maximize_first: bool,
}

// ============================================================================
// ParseContext — 解析上下文
// ============================================================================

/// `tre_parse` 使用的解析上下文。
///
/// # Rust 设计优势
///
/// - C 的实现使用裸指针追踪解析位置；Rust 使用切片 + 偏移量
/// - C 的自定义 `tre_stack_t` 被 `Vec<StackItem>` 替代
/// - C 的 `start` / `s` 裸指针被生命周期关联的切片引用替代
#[derive(Debug)]
pub(crate) struct ParseContext<'a> {
    /// TRE 内存分配器（用于 AST 节点分配）。
    pub mem: &'a mut TreMem,
    /// 显式解析栈（替代 C 的 `tre_stack_t`）。
    pub stack: Vec<StackItem>,
    /// 解析结果 AST 根节点（None = 解析中/解析失败）。
    pub result: Option<Box<AstNode>>,
    /// 剩余待解析的字节切片。
    pub pos: &'a [u8],
    /// 原始正则表达式（用于错误报告位置计算）。
    pub start: &'a [u8],
    /// 当前子匹配 ID 计数器。
    pub submatch_id: u32,
    /// 当前位置序号计数器。
    pub position: u32,
    /// 最大反向引用编号。
    pub max_backref: u32,
    /// 反向引用有效位掩码。
    pub backref_ok: u32,
    /// 编译标志（REG_EXTENDED | REG_ICASE | REG_NEWLINE | REG_NOSUB）。
    pub cflags: c_int,
}

// ============================================================================
// ast_new_literal — 创建字面量节点
// ============================================================================

/// 创建一个 LITERAL 类型的 AST 节点。
///
/// # 前置条件
///
/// - `mem` 为有效的分配器
///
/// # 后置条件
///
/// - 成功：返回 `Some(Box<AstNode>)`，节点类型为 `AstType::Literal`
/// - 失败（内存不足）：`mem.failed == true`，返回 `None`
pub(crate) fn ast_new_literal(
    _mem: &mut TreMem,
    kind: LiteralKind,
    position: u32,
) -> Option<Box<AstNode>> {
    // C 使用 tre_mem_calloc 分配；Rust 使用 Box 自动堆分配。
    let is_special = matches!(kind, LiteralKind::Empty | LiteralKind::Assertion(_) | LiteralKind::Tag(_));
    let lit = Literal {
        kind,
        position: if is_special { None } else { Some(position) },
        class: None,
        neg_classes: None,
    };
    Some(Box::new(AstNode {
        node_type: AstType::Literal,
        nullable: None,
        submatch_id: None,
        num_submatches: 0,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Literal(lit),
    }))
}

// ============================================================================
// ast_new_catenation — 创建连接节点
// ============================================================================

/// 创建 CATENATION 节点。
///
/// 若 `left` 为 `None`，直接返回 `right`（优化空连接）。
pub(crate) fn ast_new_catenation(
    _mem: &mut TreMem,
    left: Option<Box<AstNode>>,
    right: Box<AstNode>,
) -> Option<Box<AstNode>> {
    let left = match left {
        Some(l) => l,
        None => return Some(right), // 空连接优化：直接返回右子
    };
    let num_submatches = left.num_submatches + right.num_submatches;
    let cat = Catenation { left, right };
    Some(Box::new(AstNode {
        node_type: AstType::Catenation,
        nullable: None,
        submatch_id: None,
        num_submatches,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Catenation(cat),
    }))
}

// ============================================================================
// ast_new_iter — 创建迭代节点
// ============================================================================

/// 创建 ITERATION 节点包装子表达式。
///
/// `max = -1` 表示无上限，`minimal = true` 表示非贪婪。
pub(crate) fn ast_new_iter(
    _mem: &mut TreMem,
    arg: Box<AstNode>,
    min: i32,
    max: i32,
    minimal: bool,
) -> Option<Box<AstNode>> {
    let num_submatches = arg.num_submatches;
    let iter = Iteration {
        arg,
        min,
        max,
        minimal,
    };
    Some(Box::new(AstNode {
        node_type: AstType::Iteration,
        nullable: None, // 将由 tre_compute_nfl 设置
        submatch_id: None,
        num_submatches,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Iteration(iter),
    }))
}

// ============================================================================
// ast_new_union — 创建并集节点
// ============================================================================

/// 创建 UNION 节点。
///
/// 若 `left` 为 `None`，直接返回 `right`。
pub(crate) fn ast_new_union(
    _mem: &mut TreMem,
    left: Option<Box<AstNode>>,
    right: Box<AstNode>,
) -> Option<Box<AstNode>> {
    let left = match left {
        Some(l) => l,
        None => return Some(right), // 空左并集优化
    };
    let num_submatches = left.num_submatches + right.num_submatches;
    let uni = Union { left, right };
    Some(Box::new(AstNode {
        node_type: AstType::Union,
        nullable: None,
        submatch_id: None,
        num_submatches,
        num_tags: 0,
        firstpos: None,
        lastpos: None,
        obj: AstNodeObj::Union(uni),
    }))
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::format;
    use super::*;
    use super::super::tre_mem::tre_mem_new;

    // ---- AstType 测试 ----

    test!("test_ast_type_values" {
        // 验证枚举值互相区分
        assert_ne!(AstType::Literal, AstType::Catenation);
        assert_ne!(AstType::Catenation, AstType::Iteration);
        assert_ne!(AstType::Iteration, AstType::Union);
    });

    test!("test_ast_type_copy_clone" {
        let t = AstType::Literal;
        let t2 = t; // Copy
        assert_eq!(t, t2);
        let t3 = t.clone(); // Clone
        assert_eq!(t, t3);
    });

    test!("test_ast_type_debug" {
        let s = format!("{:?}", AstType::Union);
        assert!(s.contains("Union"));
    });

    // ---- LiteralKind 测试 ----

    test!("test_literal_kind_char" {
        let k = LiteralKind::Char(65, 65);
        assert_eq!(k, LiteralKind::Char(65, 65));
        assert_ne!(k, LiteralKind::Char(66, 66));
    });

    test!("test_literal_kind_special_types" {
        assert_ne!(LiteralKind::Empty, LiteralKind::Char(0, 0));
        assert_ne!(LiteralKind::Assertion(1), LiteralKind::Tag(1));
        assert_ne!(LiteralKind::Backref(1), LiteralKind::Tag(1));
    });

    test!("test_literal_kind_clone" {
        let k = LiteralKind::Tag(3);
        assert_eq!(k.clone(), LiteralKind::Tag(3));
    });

    test!("test_literal_kind_debug" {
        let k = LiteralKind::Assertion(4);
        let s = format!("{:?}", k);
        assert!(s.contains("Assertion"));
        assert!(s.contains("4"));
    });

    // ---- AstNode 测试 ----

    test!("test_ast_node_literal_creation" {
        let lit = Literal {
            kind: LiteralKind::Char(65, 65),
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
        assert_eq!(node.node_type, AstType::Literal);
        assert!(node.nullable.is_none());
        assert!(node.submatch_id.is_none());
    });

    test!("test_ast_node_catenation_creation" {
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
        let cat = Catenation { left, right };
        let node = AstNode {
            node_type: AstType::Catenation,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Catenation(cat),
        };
        assert_eq!(node.node_type, AstType::Catenation);
    });

    test!("test_ast_node_iteration_creation" {
        let lit = Literal {
            kind: LiteralKind::Char(b'x' as i64, b'x' as i64),
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
        let iter = Iteration {
            arg,
            min: 0,
            max: -1,
            minimal: false,
        };
        let node = AstNode {
            node_type: AstType::Iteration,
            nullable: Some(true), // * 可匹配空串
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Iteration(iter),
        };
        assert_eq!(node.node_type, AstType::Iteration);
        assert_eq!(node.nullable, Some(true));
    });

    test!("test_ast_node_has_nfl_fields" {
        // 验证 nullable、firstpos、lastpos 字段存在且可设值
        let lit = Literal {
            kind: LiteralKind::Char(42, 42),
            position: Some(3),
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
        // 设置 nullable
        node.nullable = Some(false);
        assert_eq!(node.nullable, Some(false));
        // 设置 firstpos
        let pos = PosAndTags {
            position: 3,
            code_min: 42,
            code_max: 42,
            tags: None,
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        node.firstpos = Some(vec![pos]);
        assert!(node.firstpos.is_some());
        assert_eq!(node.firstpos.as_ref().unwrap().len(), 1);
    });

    // ---- PosAndTags 测试 ----

    test!("test_pos_and_tags_creation" {
        let pt = PosAndTags {
            position: 1,
            code_min: 0x61,
            code_max: 0x7a,
            tags: Some(vec![0, 1]),
            assertions: 0,
            class: None,
            neg_classes: None,
            backref: None,
        };
        assert_eq!(pt.position, 1);
        assert_eq!(pt.code_min, 0x61);
        assert_eq!(pt.code_max, 0x7a);
        assert!(pt.tags.is_some());
        assert_eq!(pt.tags.as_ref().unwrap().len(), 2);
        assert_eq!(pt.assertions, 0);
    });

    test!("test_pos_and_tags_with_backref" {
        let pt = PosAndTags {
            position: 2,
            code_min: 0,
            code_max: 0,
            tags: None,
            assertions: super::super::tre::ASSERT_BACKREF,
            class: None,
            neg_classes: None,
            backref: Some(1),
        };
        assert_eq!(pt.backref, Some(1));
        assert_ne!(pt.assertions & super::super::tre::ASSERT_BACKREF, 0);
    });

    // ---- LiteralsBuilder 测试 ----

    test!("test_literals_builder_empty" {
        let lb = LiteralsBuilder {
            literals: Vec::new(),
        };
        assert!(lb.literals.is_empty());
    });

    test!("test_literals_builder_with_items" {
        let mut lb = LiteralsBuilder {
            literals: Vec::new(),
        };
        lb.literals.push(Literal {
            kind: LiteralKind::Char(65, 65),
            position: Some(1),
            class: None,
            neg_classes: None,
        });
        lb.literals.push(Literal {
            kind: LiteralKind::Char(66, 66),
            position: Some(2),
            class: None,
            neg_classes: None,
        });
        assert_eq!(lb.literals.len(), 2);
    });

    // ---- NegCollector 测试 ----

    test!("test_neg_collector_empty" {
        let nc = NegCollector {
            negate: false,
            classes: Vec::new(),
        };
        assert!(!nc.negate);
        assert!(nc.classes.is_empty());
    });

    test!("test_neg_collector_with_classes" {
        let nc = NegCollector {
            negate: true,
            classes: vec![1, 2, 3, 0],
        };
        assert!(nc.negate);
        assert_eq!(nc.classes.len(), 4);
        assert_eq!(nc.classes[3], 0); // 0 结尾
    });

    // ---- ParseContext 测试 ----

    test!("test_parse_context_creation" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"abc";
        let ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 0,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 0,
        };
        assert!(ctx.result.is_none());
        assert_eq!(ctx.pos, pattern);
        assert_eq!(ctx.start, pattern);
        assert_eq!(ctx.submatch_id, 0);
        assert_eq!(ctx.position, 0);
    });

    // ---- ast_new_literal 测试 ----

    test!("test_ast_new_literal_basic" {
        let mut mem = tre_mem_new();
        let node = ast_new_literal(&mut mem, LiteralKind::Char(65, 65), 1);
        // 在实现完成前，测试接口可调用
        // 实现后：
        // assert!(node.is_some());
        // assert_eq!(node.unwrap().node_type, AstType::Literal);
    });

    // ---- ast_new_catenation 测试 ----

    test!("test_ast_new_catenation_none_left" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(66, 66),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let right = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        });
        // left 为 None 时应直接返回 right
        let result = ast_new_catenation(&mut mem, None, right);
        // 实现后验证返回的即为原来的 right
    });

    // ---- ast_new_iter 测试 ----

    test!("test_ast_new_iter_zero_or_more" {
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
        // 创建 a* (min=0, max=-1, minimal=false)
        let result = ast_new_iter(&mut mem, arg, 0, -1, false);
        // 实现后验证
    });

    test!("test_ast_new_iter_non_greedy" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'x' as i64, b'x' as i64),
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
        // 创建 x*? (min=0, max=-1, minimal=true)
        let result = ast_new_iter(&mut mem, arg, 0, -1, true);
        // 实现后验证 minimal 标志
    });

    test!("test_ast_new_iter_exact_count" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'z' as i64, b'z' as i64),
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
        // 创建 z{3} (min=3, max=3, minimal=false)
        let result = ast_new_iter(&mut mem, arg, 3, 3, false);
        // 实现后验证
    });

    // ---- ast_new_union 测试 ----

    test!("test_ast_new_union_none_left" {
        let mut mem = tre_mem_new();
        let lit = Literal {
            kind: LiteralKind::Char(b'c' as i64, b'c' as i64),
            position: Some(2),
            class: None,
            neg_classes: None,
        };
        let right = Box::new(AstNode {
            node_type: AstType::Literal,
            nullable: None,
            submatch_id: None,
            num_submatches: 0,
            num_tags: 0,
            firstpos: None,
            lastpos: None,
            obj: AstNodeObj::Literal(lit),
        });
        // left 为 None 时应直接返回 right
        let result = ast_new_union(&mut mem, None, right);
        // 实现后验证
    });

    // ---- CopyFlags 测试 ----

    test!("test_copy_flags_default" {
        let flags = CopyFlags {
            remove_tags: false,
            maximize_first: false,
        };
        assert!(!flags.remove_tags);
        assert!(!flags.maximize_first);
    });

    test!("test_copy_flags_remove_tags" {
        let flags = CopyFlags {
            remove_tags: true,
            maximize_first: false,
        };
        assert!(flags.remove_tags);
    });
}
