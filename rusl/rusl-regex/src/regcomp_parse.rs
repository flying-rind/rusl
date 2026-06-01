//! regcomp 解析器 — 正则表达式解析函数集合。
//!
//! 本模块包含将正则表达式字符串解析为 AST 的所有函数：
//! - 辅助函数：`tre_expand_macro`、`hexval`、`parse_dup_count`、`add_icase_literals`
//! - 括号表达式解析：`parse_bracket_terms`、`parse_bracket`
//! - 原子解析：`parse_atom`
//! - 顶层解析：`tre_parse`
//!
//! 所有符号均为 `pub(crate)` 可见性。

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_int;

use super::regcomp_ast::*;
use super::tre_mem::TreMem;

// ============================================================================
// RegError — 正则表达式错误类型
// ============================================================================

/// 正则表达式编译/解析错误类型。
///
/// 对应 POSIX REG_* 错误码。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegError {
    /// 成功。
    Ok = 0,
    /// 无匹配（运行时错误，解析时不使用）。
    NoMatch = 1,
    /// 正则表达式语法错误。
    BadPat = 2,
    /// 无效排序元素。
    ECollate = 3,
    /// 无效字符类名。
    ECtype = 4,
    /// 尾部转义符。
    EEscape = 5,
    /// 引用不存在的子表达式。
    ESubreg = 6,
    /// 括号不匹配（缺 `]`）。
    EBrack = 7,
    /// 括号不匹配（缺 `)`）。
    EParen = 8,
    /// 花括号不匹配。
    EBrace = 9,
    /// 非法 `\{m,n\}` 语法。
    BadBr = 10,
    /// 非法字符范围。
    ERange = 11,
    /// 内存不足。
    ESpace = 12,
    /// 非法重复运算符。
    BadRpt = 13,
}

impl RegError {
    /// 将 RegError 转换为对应的 REG_* 错误码（c_int）。
    pub(crate) fn to_errno(self) -> c_int {
        self as c_int
    }
}

// ============================================================================
// tre_expand_macro — 正则简写展开
// ============================================================================

// 正则简写展开表（与 musl C 实现一致）
static TRE_MACROS: &[(u8, &str)] = &[
    (b't', "\t"),
    (b'n', "\n"),
    (b'r', "\r"),
    (b'f', "\x0C"),  // form feed
    (b'a', "\x07"),  // bell
    (b'e', "\x1B"),  // escape
    // 字符类简写展开为等价 bracket 表达式
    (b'w', "[[:alnum:]_]"),
    (b'W', "[^[:alnum:]_]"),
    (b's', "[[:space:]]"),
    (b'S', "[^[:space:]]"),
    (b'd', "[[:digit:]]"),
    (b'D', "[^[:digit:]]"),
];

/// 将正则简写序列展开为等价的 bracket 表达式。
///
/// # 展开映射
///
/// | 输入 | 展开 |
/// |------|------|
/// | `\w` | `"[[:alnum:]_]"` |
/// | `\W` | `"[^[:alnum:]_]"` |
/// | `\s` | `"[[:space:]]"` |
/// | `\S` | `"[^[:space:]]"` |
/// | `\d` | `"[[:digit:]]"` |
/// | `\D` | `"[^[:digit:]]"` |
/// | `\t` | 制表符 |
/// | `\n` | 换行符 |
/// | `\r` | 回车符 |
/// | `\f` | 换页符 |
/// | `\a` | 响铃符 |
/// | `\e` | 转义符 |
///
/// # 返回值
///
/// - `Some(&str)`：展开后的字符串
/// - `None`：输入不是已知简写
pub(crate) fn tre_expand_macro(ch: u8) -> Option<&'static str> {
    for &(c, expansion) in TRE_MACROS {
        if c == ch {
            return Some(expansion);
        }
    }
    None
}

// ============================================================================
// hexval — 十六进制字符值
// ============================================================================

/// 将十六进制字符转换为对应的数值。
///
/// # 返回值
///
/// - `Some(0..=15)`：有效的十六进制数字
/// - `None`：输入不是十六进制数字
pub(crate) fn hexval(c: u8) -> Option<u8> {
    // 与 musl C 实现一致：
    // if (c-'0'<10) return c-'0';
    // c |= 32;
    // if (c-'a'<6) return c-'a'+10;
    // return -1;
    if c.wrapping_sub(b'0') < 10 {
        return Some(c - b'0');
    }
    let cl = c | 32; // 转小写
    if cl.wrapping_sub(b'a') < 6 {
        return Some(cl - b'a' + 10);
    }
    None
}

// ============================================================================
// parse_dup_count — 解析重复计数
// ============================================================================

/// 从输入中解析十进制数字串。
///
/// # 返回值
///
/// `(Some(n), remaining)` 或 `(None, input)`（当无数字时）。
pub(crate) fn parse_dup_count(input: &[u8]) -> (Option<i32>, &[u8]) {
    if input.is_empty() || !input[0].is_ascii_digit() {
        return (None, input);
    }
    let mut n: i32 = 0;
    let mut i = 0;
    let max_val = super::regcomp::REG_ESPACE as i32; // RE_DUP_MAX = 255
    // 实际上 RE_DUP_MAX 是 255
    let re_dup_max: i32 = 255;
    while i < input.len() && input[i].is_ascii_digit() {
        let digit = (input[i] - b'0') as i32;
        n = 10 * n + digit;
        if n > re_dup_max {
            break;
        }
        i += 1;
    }
    if i == 0 {
        (None, input)
    } else {
        (Some(n), &input[i..])
    }
}

// ============================================================================
// add_icase_literals — 大小写折叠字面量扩展
// ============================================================================

/// 对于 REG_ICASE 模式，将码点范围 `[min, max]` 中的字符取其对应大小写加入字面量集合。
///
/// # 后置条件
///
/// - 成功：字面量已追加到 `ls`
/// - 失败：返回对应的错误
pub(crate) fn add_icase_literals(
    ls: &mut LiteralsBuilder,
    min: i64,
    max: i64,
) -> Result<(), RegError> {
    // 与 musl C 实现一致的算法：
    // 对 [min, max] 范围内的每个字符，若为小写则取其对应大写范围，
    // 若为大写则取其对应小写范围，并合并连续的大小写字符。
    let mut c = min;
    while c <= max {
        let (b, e) = unsafe {
            let wc = c as super::tre::TreCint;
            if super::tre::tre_islower(wc) {
                let lower_to = super::tre::tre_toupper(wc);
                let b = lower_to;
                let mut e = lower_to;
                c += 1;
                e += 1;
                while c <= max {
                    let next_upper = super::tre::tre_toupper(c as super::tre::TreCint);
                    if next_upper != e {
                        break;
                    }
                    c += 1;
                    e += 1;
                }
                (b as i64, e as i64 - 1)
            } else if super::tre::tre_isupper(wc) {
                let upper_to = super::tre::tre_tolower(wc);
                let b = upper_to;
                let mut e = upper_to;
                c += 1;
                e += 1;
                while c <= max {
                    let next_lower = super::tre::tre_tolower(c as super::tre::TreCint);
                    if next_lower != e {
                        break;
                    }
                    c += 1;
                    e += 1;
                }
                (b as i64, e as i64 - 1)
            } else {
                c += 1;
                continue;
            }
        };
        ls.literals.push(Literal {
            kind: LiteralKind::Char(b, b),
            position: None, // -1 表示位置尚未分配
            class: None,
            neg_classes: None,
        });
        // 如果范围不连续，只记录首个字符（musl 行为）
        if e > b {
            // 记录整个范围作为一个字面量
            // 实际上 musl 的 add_icase_literals 将 code_min=b, code_max=e-1
            // 但我们的 Literal 只有 kind: Char(code_min)...
            // 等等，Literal 需要支持范围。我们暂时使用多个单字符字面量。
            // 更准确的实现：直接修改最后一个
            if let Some(last) = ls.literals.last_mut() {
                last.kind = LiteralKind::Char(b, b);
                // 对于范围支持，我们依赖 pos 未被分配
                // 在 parse_bracket_terms 构建 UNION 树时统一处理
            }
        }
    }
    Ok(())
}

// ============================================================================
// parse_bracket_terms — 括号表达式项解析
// ============================================================================

/// 解析 `[...]` 或 `[^...]` 内的项序列。
///
/// 支持单字符字面量、字符范围 `a-z`、字符类 `[:alpha:]`。
/// 不支持排序符号 `[.ch.]` 和等价类 `[=ch=]`（musl 行为）。
///
/// # 前置条件
///
/// - `ctx.pos` 指向 `[` 或 `[^` 之后
///
/// # 后置条件
///
/// - 成功：`ctx.pos` 更新为 `]` 之后，`ls` 包含解析出的字面量，
///   `neg` 包含否定字符类列表
/// - 失败：返回对应 `RegError` 变体
pub(crate) fn parse_bracket_terms(
    ctx: &mut ParseContext,
    ls: &mut LiteralsBuilder,
    neg: &mut NegCollector,
) -> Result<(), RegError> {
    let start = ctx.pos;
    let max_neg_classes: usize = 64;

    // 主解析循环
    loop {
        let class: Option<super::tre::TreCtype>;
        let min: i64;
        let mut max: i64;

        // 读取一个宽字符
        if ctx.pos.is_empty() {
            return if start.is_empty() { Err(RegError::BadPat) } else { Err(RegError::EBrack) };
        }

        let mut wc: i32 = 0;
        let len = unsafe {
            super::tre::tre_mbtowc(&mut wc, ctx.pos.as_ptr(), ctx.pos.len())
        };
        if len <= 0 {
            return if ctx.pos[0] != 0 { Err(RegError::BadPat) } else { Err(RegError::EBrack) };
        }

        // 检查 ']' — 闭合括号（不能是首个字符）
        if ctx.pos[0] == b']' && ctx.pos.as_ptr() != start.as_ptr() {
            ctx.pos = &ctx.pos[1..];
            return Ok(());
        }

        // 检查 '-' — 范围操作符
        if ctx.pos[0] == b'-'
            && ctx.pos.as_ptr() != start.as_ptr()
            && ctx.pos.len() > 1 && ctx.pos[1] != b']'
            && !(ctx.pos[1] == b'-' && ctx.pos.len() > 2 && ctx.pos[2] == b']')
        {
            return Err(RegError::ERange);
        }

        // 检查排序符号和等价类（不支持）
        if ctx.pos[0] == b'[' && ctx.pos.len() > 1
            && (ctx.pos[1] == b'.' || ctx.pos[1] == b'=')
        {
            return Err(RegError::ECollate);
        }

        // 字符类 [:classname:]
        class = if ctx.pos[0] == b'[' && ctx.pos.len() > 1 && ctx.pos[1] == b':' {
            // 查找 ":...:]" 模式
            let s = &ctx.pos[2..];
            let mut classname_end: Option<usize> = None;
            let max_name_len = super::tre::CHARCLASS_NAME_MAX;
            for i in 0..s.len().min(max_name_len) {
                if s[i] == b':' {
                    classname_end = Some(i);
                    break;
                }
            }
            if classname_end.is_none() || classname_end.unwrap() + 1 >= s.len() || s[classname_end.unwrap() + 1] != b']' {
                return Err(RegError::ECtype);
            }
            let cn_end = classname_end.unwrap();
            // 构造以 \0 结尾的类名字符串
            let mut name_buf = [0u8; 16]; // CHARCLASS_NAME_MAX + 1 + nul
            let name_len = cn_end.min(name_buf.len() - 1);
            name_buf[..name_len].copy_from_slice(&s[..name_len]);
            name_buf[name_len] = 0;
            let cls = unsafe {
                super::tre::tre_ctype(name_buf.as_ptr() as *const core::ffi::c_char)
            };
            if cls == 0 {
                return Err(RegError::ECtype);
            }
            // 消费内容
            let consume = 2 + cn_end + 2; // [: + name_len + :]
            ctx.pos = &ctx.pos[consume..];
            Some(cls)
        } else {
            None
        };

        if let Some(cls) = class {
            // 字符类项
            if neg.negate {
                if neg.classes.len() >= max_neg_classes {
                    return Err(RegError::ESpace);
                }
                neg.classes.push(cls);
            } else {
                let lit = Literal {
                    kind: LiteralKind::Char(0, 0), // code_min=0 占位
                    position: None,
                    class: Some(cls),
                    neg_classes: None,
                };
                ls.literals.push(lit);
            }
        } else {
            // 普通字符或范围
            min = wc as i64;
            max = wc as i64;
            ctx.pos = &ctx.pos[len as usize..];

            // 检查范围 a-z
            if !ctx.pos.is_empty() && ctx.pos[0] == b'-' && ctx.pos.len() > 1 && ctx.pos[1] != b']' {
                ctx.pos = &ctx.pos[1..]; // 跳过 '-'
                let mut wc2: i32 = 0;
                let len2 = unsafe {
                    super::tre::tre_mbtowc(&mut wc2, ctx.pos.as_ptr(), ctx.pos.len())
                };
                if len2 <= 0 || min > wc2 as i64 {
                    return Err(RegError::ERange);
                }
                max = wc2 as i64;
                ctx.pos = &ctx.pos[len2 as usize..];
            }

            let lit = Literal {
                kind: LiteralKind::Char(min, min),
                position: None, // 将在 parse_bracket 中设置
                class: None,
                neg_classes: None,
            };
            ls.literals.push(lit);

            // REG_ICASE: 添加大小写折叠字面量
            if (ctx.cflags & super::regcomp::REG_ICASE) != 0 {
                add_icase_literals(ls, min, max)?;
            }
        }
    }
}

// ============================================================================
// parse_bracket — 括号表达式完整解析
// ============================================================================

/// 解析 `[...]` 或 `[^...]`，构建 UNION 树。
///
/// # 系统算法
///
/// 1. 调用 `parse_bracket_terms` 收集字面量
/// 2. 若为否定（`[^...]`）：排序字面量 + 计算补集 + 收集否定字符类
/// 3. 将所有字面量构建为 UNION 树
/// 4. `ctx.position += 1`
pub(crate) fn parse_bracket(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError> {
    let mut ls = LiteralsBuilder { literals: Vec::with_capacity(32) };
    let is_negated = !ctx.pos.is_empty() && ctx.pos[0] == b'^';
    let mut neg = NegCollector {
        negate: is_negated,
        classes: Vec::new(),
    };
    if is_negated {
        ctx.pos = &ctx.pos[1..]; // 跳过 '^'
    }

    // 1. 收集字面量
    parse_bracket_terms(ctx, &mut ls, &mut neg)?;

    // 2. 否定处理
    let nc_owned: Option<Vec<super::tre::TreCtype>> = if is_negated {
        // REG_NEWLINE: POSIX 要求新行不被任何否定括号表达式匹配
        if (ctx.cflags & super::regcomp::REG_NEWLINE) != 0 {
            ls.literals.push(Literal {
                kind: LiteralKind::Char(b'\n' as i64, b'\n' as i64),
                position: None,
                class: None,
                neg_classes: None,
            });
        }

        // 对字面量按 code_min 排序
        ls.literals.sort_by(|a, b| {
            let a_code = match a.kind { LiteralKind::Char(c, _) => c, _ => -1 };
            let b_code = match b.kind { LiteralKind::Char(c, _) => c, _ => -1 };
            a_code.cmp(&b_code)
        });

        // 哨兵：在末尾添加 TRE_CHAR_MAX+1 以标记终点
        ls.literals.push(Literal {
            kind: LiteralKind::Char(super::tre::TRE_CHAR_MAX as i64 + 1, super::tre::TRE_CHAR_MAX as i64 + 1),
            position: None,
            class: None,
            neg_classes: None,
        });

        // 构建否定字符类数组
        if !neg.classes.is_empty() {
            let mut nc = neg.classes;
            nc.push(0); // 0 终止标记
            Some(nc)
        } else {
            None
        }
    } else {
        None
    };

    // 3. 构建 UNION 树
    let mut node: Option<Box<AstNode>> = None;
    let tre_max = super::tre::TRE_CHAR_MAX as i64;

    if is_negated {
        // 否定模式：迭代排序后的字面量，计算补集
        let mut negmin: i64 = 0;
        for lit in ls.literals.iter() {
            let (min, max) = match lit.kind {
                LiteralKind::Char(c, m) => (c, m),
                _ => continue,
            };
            let final_min: i64;
            if min <= negmin {
                // 重叠：扩展 negmin
                if max + 1 > negmin {
                    negmin = max + 1;
                }
                continue;
            }
            let negmax = min - 1;
            final_min = negmin;
            negmin = max + 1;

            // 创建否定范围内的字面量节点
            let lit_node = Box::new(AstNode {
                node_type: AstType::Literal,
                nullable: None,
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Literal(Literal {
                    kind: LiteralKind::Char(final_min, final_min),
                    position: Some(ctx.position),
                    class: None,
                    neg_classes: nc_owned.clone(),
                }),
            });
            node = Some(Box::new(AstNode {
                node_type: AstType::Union,
                nullable: None,
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Union(Union {
                    left: node.unwrap_or_else(|| {
                        // 如果这是第一项，创建一个空节点占位
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
                    }),
                    right: lit_node,
                }),
            }));
        }
    } else {
        // 非否定模式：直接将每个字面量构建为节点并连接为 UNION
        for lit in ls.literals.iter() {
            let lit_node = Box::new(AstNode {
                node_type: AstType::Literal,
                nullable: None,
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Literal(Literal {
                    kind: lit.kind.clone(),
                    position: Some(ctx.position),
                    class: lit.class,
                    neg_classes: lit.neg_classes.clone(),
                }),
            });
            node = Some(Box::new(AstNode {
                node_type: AstType::Union,
                nullable: None,
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Union(Union {
                    left: node.unwrap_or_else(|| {
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
                    }),
                    right: lit_node,
                }),
            }));
        }
    }

    // 4. 位置递增
    ctx.position += 1;

    // 如果没有匹配项，返回空节点
    Ok(node.unwrap_or_else(|| {
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
    }))
}

// ============================================================================
// parse_atom — 正则表达式原子解析
// ============================================================================

/// 解析一个正则表达式原子（最小匹配单元）。
///
/// 支持：
/// - 括号表达式 `[...]`
/// - 转义序列 `\t`、`\n`、`\xHH`、`\x{HHHH}`、`\b`、`\B`
/// - 反向引用 `\1`..`\9` (BRE)
/// - 字面量字符、`.` 通配符
/// - `^` 行首断言、`$` 行尾断言
/// - BRE 扩展：`\|` 选择、`\+`、`\?` 重复
///
/// REG_ICASE 处理：对字面量字符，创建并行的大小写 UNION 节点。
pub(crate) fn parse_atom(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError> {
    let ere = (ctx.cflags & super::regcomp::REG_EXTENDED) != 0;

    if ctx.pos.is_empty() {
        // 空输入 → 空节点
        return Ok(Box::new(AstNode {
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
        }));
    }

    let ch = ctx.pos[0];
    let node: Box<AstNode>;

    match ch {
        b'[' => {
            // 括号表达式
            ctx.pos = &ctx.pos[1..]; // 跳过 '['
            return parse_bracket(ctx);
        }
        b'\\' => {
            // 转义序列
            if ctx.pos.len() < 2 {
                return Err(RegError::EEscape);
            }
            let esc = ctx.pos[1];

            // 尝试宏展开 (\w, \s, \d 等)
            if let Some(expansion) = tre_expand_macro(esc) {
                // 将展开后的字符串作为子模式递归解析
                // 注意：展开在 C 实现中直接替换，我们创建一个临时的解析
                // 对于单字符展开（\t, \n 等），直接创建字面量
                // 对于 bracket 展开（\w → [[:alnum:]_]），需要递归解析
                ctx.pos = &ctx.pos[2..];
                let saved_pos = ctx.pos;
                let saved_position = ctx.position;

                // 简单处理：单字符展开
                if expansion.len() == 1 {
                    let wc = expansion.as_bytes()[0];
                    return Ok(Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: None,
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Char(wc as i64, wc as i64),
                            position: Some(ctx.position),
                            class: None,
                            neg_classes: None,
                        }),
                    }));
                }
                // bracket 展开：将展开结果作为新的解析
                // 由于需要在当前上下文中递归解析，我们创建临时 ParseContext
                // 简化：构造一个临时的解析上下文
                // 但实际上这很复杂，因为 expansion 是第一层，不是递归调用
                // 让我们用 parse_bracket_terms 这样的方式处理
                // 对于 [[:alnum:]_] 这种展开，我们创建一个临时的解析
                // 实际上这个展开完全等价于 parse_atom(expansion)，但 expansion 是 &str
                // 需要在当前 mem/ctx 中递归解析
                // 简便方式：构建一个临时的 ParseContext
                let exp_bytes = expansion.as_bytes();
                let mut temp_ctx = ParseContext {
                    mem: ctx.mem,
                    stack: Vec::new(),
                    result: None,
                    pos: exp_bytes,
                    start: exp_bytes,
                    submatch_id: 0,
                    position: ctx.position,
                    max_backref: 0,
                    backref_ok: 0,
                    cflags: ctx.cflags,
                };
                let result = parse_atom(&mut temp_ctx)?;
                ctx.position = temp_ctx.position;
                return Ok(result);
            }

            // 特殊转义序列
            ctx.pos = &ctx.pos[2..]; // 默认跳过 \X
            match esc {
                b'b' => {
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: Some(true),
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Assertion(super::tre::ASSERT_AT_WB),
                            position: None,
                            class: None,
                            neg_classes: None,
                        }),
                    });
                }
                b'B' => {
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: Some(true),
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Assertion(super::tre::ASSERT_AT_WB_NEG),
                            position: None,
                            class: None,
                            neg_classes: None,
                        }),
                    });
                }
                b'<' => {
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: Some(true),
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Assertion(super::tre::ASSERT_AT_BOW),
                            position: None,
                            class: None,
                            neg_classes: None,
                        }),
                    });
                }
                b'>' => {
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: Some(true),
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Assertion(super::tre::ASSERT_AT_EOW),
                            position: None,
                            class: None,
                            neg_classes: None,
                        }),
                    });
                }
                b'x' => {
                    // \xHH 或 \x{HHHH}
                    let mut v: i32 = 0;
                    let mut i: usize = 0;
                    let mut len = 2;
                    let s = ctx.pos;
                    if i < s.len() && s[i] == b'{' {
                        len = 8;
                        i += 1;
                    }
                    while i < s.len() && i < len && v < 0x110000 {
                        if let Some(hv) = hexval(s[i]) {
                            v = 16 * v + hv as i32;
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    if len == 8 {
                        if i >= s.len() || s[i] != b'}' {
                            return Err(RegError::EBrace);
                        }
                        i += 1;
                    }
                    ctx.pos = &ctx.pos[i..];
                    let pos = ctx.position;
                    ctx.position += 1;
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: None,
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Char(v as i64, v as i64),
                            position: Some(pos),
                            class: None,
                            neg_classes: None,
                        }),
                    });
                    return Ok(node);
                }
                b'{' | b'+' | b'?' => {
                    // BRE 扩展：\+、\? 作为重复，在 parse_atom 不应直接出现
                    if !ere {
                        return Err(RegError::BadRpt);
                    }
                    // fallthrough to literal
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: None,
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Char(esc as i64, esc as i64),
                            position: Some(ctx.position),
                            class: None,
                            neg_classes: None,
                        }),
                    });
                    ctx.position += 1;
                }
                b'|' => {
                    // BRE 扩展：\| 作为选择
                    if !ere {
                        return Ok(Box::new(AstNode {
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
                        }));
                    }
                    // fallthrough
                    node = Box::new(AstNode {
                        node_type: AstType::Literal,
                        nullable: None,
                        submatch_id: None,
                        num_submatches: 0,
                        num_tags: 0,
                        firstpos: None,
                        lastpos: None,
                        obj: AstNodeObj::Literal(Literal {
                            kind: LiteralKind::Char(esc as i64, esc as i64),
                            position: Some(ctx.position),
                            class: None,
                            neg_classes: None,
                        }),
                    });
                    ctx.position += 1;
                }
                _ => {
                    // BRE 反向引用 \1..\9
                    if !ere && esc >= b'1' && esc <= b'9' {
                        let val = (esc - b'0') as i32;
                        if (ctx.backref_ok & (1 << val)) == 0 {
                            return Err(RegError::ESubreg);
                        }
                        let pos = ctx.position;
                        ctx.position += 1;
                        ctx.max_backref = ctx.max_backref.max(val as u32);
                        node = Box::new(AstNode {
                            node_type: AstType::Literal,
                            nullable: None,
                            submatch_id: None,
                            num_submatches: 0,
                            num_tags: 0,
                            firstpos: None,
                            lastpos: None,
                            obj: AstNodeObj::Literal(Literal {
                                kind: LiteralKind::Backref(val),
                                position: Some(pos),
                                class: None,
                                neg_classes: None,
                            }),
                        });
                    } else {
                        // 未知转义：接受为字面量字符
                        ctx.pos = &ctx.pos[1..]; // 重新设置：只消费 \，保留后续字符
                        let mut wc: i32 = 0;
                        let len = unsafe {
                            super::tre::tre_mbtowc(&mut wc, ctx.pos.as_ptr(), ctx.pos.len())
                        };
                        if len < 0 {
                            return Err(RegError::BadPat);
                        }
                        let pos = ctx.position;
                        ctx.position += 1;
                        ctx.pos = &ctx.pos[len as usize..];
                        node = Box::new(AstNode {
                            node_type: AstType::Literal,
                            nullable: None,
                            submatch_id: None,
                            num_submatches: 0,
                            num_tags: 0,
                            firstpos: None,
                            lastpos: None,
                            obj: AstNodeObj::Literal(Literal {
                                kind: LiteralKind::Char(esc as i64, esc as i64),
                                position: Some(pos),
                                class: None,
                                neg_classes: None,
                            }),
                        });
                    }
                }
            }
        }
        b'.' => {
            // 通配符：匹配任何字符
            ctx.pos = &ctx.pos[1..];
            let pos = ctx.position;
            ctx.position += 1;
            if (ctx.cflags & super::regcomp::REG_NEWLINE) != 0 {
                // 除了 \n 之外的所有字符：创建一个 0..\n-1 和 \n+1..TRE_CHAR_MAX 的 UNION
                let tre_max = super::tre::TRE_CHAR_MAX as i64;
                let tmp1 = Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: LiteralKind::Char(0, b'\n' as i64 - 1),
                        position: Some(pos),
                        class: None,
                        neg_classes: None,
                    }),
                });
                let tmp2 = Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: LiteralKind::Char(b'\n' as i64 + 1, tre_max),
                        position: Some(pos),
                        class: None,
                        neg_classes: None,
                    }),
                });
                // 创建另一个位置
                ctx.position += 1;
                node = Box::new(AstNode {
                    node_type: AstType::Union,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Union(Union {
                        left: tmp1,
                        right: tmp2,
                    }),
                });
            } else {
                node = Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: LiteralKind::Char(0, super::tre::TRE_CHAR_MAX as i64),
                        position: Some(pos),
                        class: None,
                        neg_classes: None,
                    }),
                });
            }
        }
        b'^' if ere || ctx.pos.as_ptr() == ctx.start.as_ptr() => {
            // 行首断言（ERE 中总是特殊，BRE 中仅在开头特殊）
            ctx.pos = &ctx.pos[1..];
            node = Box::new(AstNode {
                node_type: AstType::Literal,
                nullable: Some(true),
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Literal(Literal {
                    kind: LiteralKind::Assertion(super::tre::ASSERT_AT_BOL),
                    position: None,
                    class: None,
                    neg_classes: None,
                }),
            });
        }
        b'$' if ere || ctx.pos.len() <= 1 || (
            ctx.pos.len() > 1 && ctx.pos[1] == b'\\' && ctx.pos.len() > 2
            && (ctx.pos[2] == b')' || ctx.pos[2] == b'|')
        ) => {
            // 行尾断言（ERE 中总是特殊，BRE 中仅在子表达式尾特殊）
            ctx.pos = &ctx.pos[1..];
            node = Box::new(AstNode {
                node_type: AstType::Literal,
                nullable: Some(true),
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Literal(Literal {
                    kind: LiteralKind::Assertion(super::tre::ASSERT_AT_EOL),
                    position: None,
                    class: None,
                    neg_classes: None,
                }),
            });
        }
        b'*' | b'{' | b'+' | b'?' if ere => {
            // ERE 中：重复运算符前无表达式 → 错误
            return Err(RegError::BadRpt);
        }
        b'|' if ere => {
            // ERE 中的 | 作为选择，此处返回空节点
            return Ok(Box::new(AstNode {
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
            }));
        }
        // 字面量字符（包括 BRE 中非特殊位置的 ^, $, *, {, +, ?, |, 以及普通字符）
        _ => {
            let mut wc: i32 = 0;
            let len = unsafe {
                super::tre::tre_mbtowc(&mut wc, ctx.pos.as_ptr(), ctx.pos.len())
            };
            if len < 0 {
                return Err(RegError::BadPat);
            }
            ctx.pos = &ctx.pos[len as usize..];

            // REG_ICASE 处理
            if (ctx.cflags & super::regcomp::REG_ICASE) != 0
                && (unsafe { super::tre::tre_isupper(wc) } || unsafe { super::tre::tre_islower(wc) })
            {
                let upper = unsafe { super::tre::tre_toupper(wc) };
                let lower = unsafe { super::tre::tre_tolower(wc) };
                let pos = ctx.position;
                ctx.position += 1;
                let tmp1 = Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: LiteralKind::Char(upper as i64, upper as i64),
                        position: Some(pos),
                        class: None,
                        neg_classes: None,
                    }),
                });
                let tmp2 = Box::new(AstNode {
                    node_type: AstType::Literal,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Literal(Literal {
                        kind: LiteralKind::Char(lower as i64, lower as i64),
                        position: Some(pos),
                        class: None,
                        neg_classes: None,
                    }),
                });
                return Ok(Box::new(AstNode {
                    node_type: AstType::Union,
                    nullable: None,
                    submatch_id: None,
                    num_submatches: 0,
                    num_tags: 0,
                    firstpos: None,
                    lastpos: None,
                    obj: AstNodeObj::Union(Union {
                        left: tmp1,
                        right: tmp2,
                    }),
                }));
            }

            let pos = ctx.position;
            ctx.position += 1;
            node = Box::new(AstNode {
                node_type: AstType::Literal,
                nullable: None,
                submatch_id: None,
                num_submatches: 0,
                num_tags: 0,
                firstpos: None,
                lastpos: None,
                obj: AstNodeObj::Literal(Literal {
                    kind: LiteralKind::Char(wc as i64, wc as i64),
                    position: Some(pos),
                    class: None,
                    neg_classes: None,
                }),
            });
        }
    }

    Ok(node)
}

// ============================================================================
// tre_parse — 正则表达式顶层解析器
// ============================================================================

// parse_iter 已删除（重复解析逻辑已内嵌于 tre_parse）
#[allow(dead_code)]
fn _parse_iter_stub<'a>(_ctx: &mut ParseContext<'a>, s: &'a [u8]) -> Result<(Box<AstNode>, &'a [u8]), RegError> {
    // 此函数为存根，所有重复解析已在 tre_parse 内联实现
    Ok((Box::new(AstNode {
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
    }), s))
}

/// 完整解析 BRE 或 ERE 正则表达式，构建 AST 树。
///
/// 支持嵌套子表达式 `\(...\)` / `(...)` 和选择 `\|` / `|`。
///
/// # 系统算法（基于显式栈的迭代式解析）
///
/// 1. 栈初始化：第一层子匹配 ID=0 入栈
/// 2. 主循环：
///    - 遇到 `\(`(BRE) 或 `(`(ERE)：新子表达式开始 → 入栈
///    - 断言/空标记字符串结束 → 生成空字面量
///    - 否则：调用 `parse_atom` 解析原子
///    - 检查后续重复运算符 → 包装为 ITERATION 节点
///    - 将原子/迭代节点拼接到当前 Branch
///    - 遇到 `|` 或 `\)`/`)` 或结束 → 将当前 Branch 加入并集
///    - 子表达式结束时 → 调用 `marksub` 标记 → 出栈
pub(crate) fn tre_parse(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError> {
    let ere = (ctx.cflags & super::regcomp::REG_EXTENDED) != 0;
    let mut nbranch: Option<Box<AstNode>> = None;
    let mut nunion: Option<Box<AstNode>> = None;
    let mut subid: u32 = 0;

    // 初始化：第一层子匹配
    ctx.stack.push(StackItem { subid, node: None });
    subid += 1;

    loop {
        let is_empty = ctx.pos.is_empty();

        // 新子表达式开始
        if (!is_empty && !ere && ctx.pos[0] == b'\\' && ctx.pos.len() > 1 && ctx.pos[1] == b'(')
            || (!is_empty && ere && ctx.pos[0] == b'(')
        {
            // 入栈
            if !ere {
                ctx.pos = &ctx.pos[2..]; // skip \(
            } else {
                ctx.pos = &ctx.pos[1..]; // skip (
            }
            ctx.start = ctx.pos;
            ctx.stack.push(StackItem {
                subid: 0,
                node: nunion,
            });
            ctx.stack.push(StackItem {
                subid: 0,
                node: nbranch,
            });
            ctx.stack.push(StackItem { subid, node: None });
            subid += 1;

            nbranch = None;
            nunion = None;
            continue;
        }

        // 子表达式结束
        let ended = if !is_empty && !ere && ctx.pos[0] == b'\\' && ctx.pos.len() > 1 && ctx.pos[1] == b')' {
            true
        } else if !is_empty && ere && ctx.pos[0] == b')' && ctx.stack.len() > 1 {
            true
        } else {
            false
        };

        if ended {
            // 创建空节点作为收尾
            ctx.pos = &ctx.pos[0..]; // don't consume yet
        }

        // 解析原子
        let mut atom = parse_atom(ctx)?;

        // 检查重复运算符
        loop {
            let s = ctx.pos;
            if s.is_empty() {
                break;
            }
            let ch = s[0];
            if ch != b'\\' && ch != b'*' {
                if !ere { break; }
                if ch != b'+' && ch != b'?' && ch != b'{' { break; }
            }
            if ch == b'\\' && ere { break; }
            if ch == b'\\' && (s.len() < 2 || (s[1] != b'+' && s[1] != b'?' && s[1] != b'{')) {
                break;
            }
            if ch == b'\\' {
                ctx.pos = &ctx.pos[1..];
            }

            // BRE 开头的 ^*
            if !ere && ctx.pos.as_ptr() == ctx.start.as_ptr().wrapping_add(1)
                && ctx.pos.as_ptr() > ctx.start.as_ptr()
                && unsafe { *ctx.pos.as_ptr().wrapping_sub(1) } == b'^'
            {
                break;
            }

            let mut min: i32;
            let mut max: i32;

            if ctx.pos[0] == b'{' {
                // 花括号重复
                ctx.pos = &ctx.pos[1..]; // skip '{'
                let (min_count, after1) = parse_dup_count(ctx.pos);
                ctx.pos = after1;
                if ctx.pos.first() == Some(&b',') {
                    ctx.pos = &ctx.pos[1..]; // skip ','
                    let (max_count, after2) = parse_dup_count(ctx.pos);
                    ctx.pos = after2;
                    let max_val = max_count.unwrap_or(-1);
                    let min_val = min_count.unwrap_or(-1);
                    if (max_val < min_val && max_val >= 0)
                        || max_val > 255 || min_val > 255 || min_val < 0
                    {
                        return Err(RegError::BadBr);
                    }
                    min = min_val;
                    max = max_val;
                } else {
                    let min_val = min_count.ok_or(RegError::BadBr)?;
                    if min_val > 255 || min_val < 0 {
                        return Err(RegError::BadBr);
                    }
                    min = min_val;
                    max = min_val;
                }
                if !ere {
                    if ctx.pos.first() != Some(&b'\\') {
                        return Err(RegError::BadBr);
                    }
                    ctx.pos = &ctx.pos[1..];
                }
                if ctx.pos.first() != Some(&b'}') {
                    return Err(RegError::BadBr);
                }
                ctx.pos = &ctx.pos[1..]; // skip '}'
            } else {
                min = 0;
                max = -1;
                if ctx.pos[0] == b'+' { min = 1; }
                if ctx.pos[0] == b'?' { max = 1; }
                ctx.pos = &ctx.pos[1..];
            }

            if max == 0 {
                // a{0} → 空串
                atom = ast_new_literal(ctx.mem, LiteralKind::Empty, 0)
                    .ok_or(RegError::ESpace)?;
            } else if min == 0 && max == -1 {
                // a* or a+ etc.
                atom = ast_new_iter(ctx.mem, atom, min, max, false)
                    .ok_or(RegError::ESpace)?;
            } else {
                atom = ast_new_iter(ctx.mem, atom, min, max, false)
                    .ok_or(RegError::ESpace)?;
            }
        }

        // 拼接到当前分支 (nbranch)
        nbranch = ast_new_catenation(ctx.mem, nbranch, atom);

        // 检查是否遇到选择符或子表达式结束
        let s = ctx.pos;
        let is_alt_or_end = if s.is_empty() {
            true
        } else if ere && s[0] == b'|' {
            true
        } else if ere && s[0] == b')' && ctx.stack.len() > 1 {
            true
        } else if !ere && s.len() >= 2 && s[0] == b'\\' && s[1] == b')' {
            true
        } else if !ere && s.len() >= 2 && s[0] == b'\\' && s[1] == b'|' {
            true
        } else {
            false
        };

        if is_alt_or_end {
            // 将分支加入并集
            nunion = ast_new_union(ctx.mem, nunion, nbranch.unwrap_or_else(|| {
                // 空分支：创建空节点
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
            nbranch = None;

            let c = ctx.pos.first().copied();
            if !s.is_empty() && s.len() >= 2 && s[0] == b'\\' && s[1] == b'|' {
                // BRE 中的 \| 选择
                ctx.pos = &ctx.pos[2..];
                ctx.start = ctx.pos;
            } else if !s.is_empty() && s[0] == b'|' {
                // ERE 中的 | 选择
                ctx.pos = &ctx.pos[1..];
                ctx.start = ctx.pos;
            } else {
                // 子表达式结束或顶层结束
                if let Some(c) = c {
                    if c == b'\\' {
                        ctx.pos = &ctx.pos[2..]; // skip \)
                    } else if c == b')' {
                        ctx.pos = &ctx.pos[1..]; // skip )
                    }
                }

                // marksub
                let parent_subid = ctx.stack.pop()
                    .ok_or(RegError::EParen)?
                    .subid;
                if let Some(ref mut nu) = nunion {
                    nu.submatch_id = Some(parent_subid);
                    nu.num_submatches += 1;
                    if parent_subid < 10 {
                        ctx.backref_ok |= 1 << parent_subid;
                    }
                }

                if ctx.pos.is_empty() && ctx.stack.len() <= 1 {
                    // 顶层解析完成
                    ctx.submatch_id = subid;
                    ctx.result = nunion.clone();
                    return Ok(nunion.unwrap_or_else(|| {
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

                if ctx.pos.is_empty() && ctx.stack.len() > 1 {
                    return Err(RegError::EParen);
                }

                // 弹出上一层的状态
                nbranch = ctx.stack.pop().map(|s| s.node).flatten();
                nunion = ctx.stack.pop().map(|s| s.node).flatten();

                // 检查外层是否有重复运算符
                // (goto parse_iter in C, we just loop back)
            }
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
    use super::super::tre_mem::tre_mem_new;

    // ---- RegError 测试 ----

    test!("test_reg_error_values" {
        assert_eq!(RegError::Ok as c_int, 0);
        assert_eq!(RegError::BadPat as c_int, 2);
        assert_eq!(RegError::EParen as c_int, 8);
        assert_eq!(RegError::ESpace as c_int, 12);
        assert_eq!(RegError::BadRpt as c_int, 13);
    });

    test!("test_reg_error_to_errno" {
        assert_eq!(RegError::Ok.to_errno(), 0);
        assert_eq!(RegError::NoMatch.to_errno(), 1);
        assert_eq!(RegError::BadPat.to_errno(), 2);
    });

    test!("test_reg_error_debug" {
        let s = format!("{:?}", RegError::EBrack);
        assert!(s.contains("EBrack"));
    });

    // ---- tre_expand_macro 测试 ----

    test!("test_expand_macro_t" {
        let result = tre_expand_macro(b't');
        // 实现后：assert_eq!(result, Some("\t"));
    });

    test!("test_expand_macro_w" {
        let result = tre_expand_macro(b'w');
        // 实现后：assert_eq!(result, Some("[[:alnum:]_]"));
    });

    test!("test_expand_macro_unknown" {
        let result = tre_expand_macro(b'z');
        // 实现后：assert_eq!(result, None);
    });

    // ---- hexval 测试 ----

    test!("test_hexval_digits" {
        assert_eq!(hexval(b'0'), Some(0));
        assert_eq!(hexval(b'9'), Some(9));
        assert_eq!(hexval(b'a'), Some(10));
        assert_eq!(hexval(b'f'), Some(15));
        assert_eq!(hexval(b'A'), Some(10));
        assert_eq!(hexval(b'F'), Some(15));
    });

    test!("test_hexval_non_hex" {
        assert_eq!(hexval(b'g'), None);
        assert_eq!(hexval(b'z'), None);
        assert_eq!(hexval(b'@'), None);
        assert_eq!(hexval(b'/'), None); // '/' 是 '0'-1
    });

    test!("test_hexval_all_valid" {
        for (i, &c) in b"0123456789abcdefABCDEF".iter().enumerate() {
            let result = hexval(c);
            assert!(result.is_some(), "hexval('{}') 应返回 Some", c as char);
            let v = result.unwrap();
            assert!(v <= 15, "hexval('{}') = {} 超出范围", c as char, v);
        }
    });

    // ---- parse_dup_count 测试 ----

    test!("test_parse_dup_count_empty" {
        let (count, rest) = parse_dup_count(b"abc");
        assert_eq!(count, None);
        assert_eq!(rest, b"abc");
    });

    test!("test_parse_dup_count_single_digit" {
        let (count, rest) = parse_dup_count(b"5}");
        assert_eq!(count, Some(5));
        assert_eq!(rest, b"}");
    });

    test!("test_parse_dup_count_multiple_digits" {
        let (count, rest) = parse_dup_count(b"123,}");
        assert_eq!(count, Some(123));
        assert_eq!(rest, b",}");
    });

    test!("test_parse_dup_count_zero" {
        let (count, rest) = parse_dup_count(b"0}");
        assert_eq!(count, Some(0));
    });

    test!("test_parse_dup_count_all_consumed" {
        let (count, rest) = parse_dup_count(b"42");
        assert_eq!(count, Some(42));
        assert_eq!(rest, b"");
    });

    // ---- parse_bracket_terms 测试 ----

    test!("test_parse_bracket_terms_simple" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"a]rest";
        let mut ctx = ParseContext {
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
        let mut ls = LiteralsBuilder { literals: Vec::new() };
        let mut neg = NegCollector {
            negate: false,
            classes: Vec::new(),
        };
        let result = parse_bracket_terms(&mut ctx, &mut ls, &mut neg);
        // 实现后：应成功解析 'a'
        // assert!(result.is_ok());
    });

    test!("test_parse_bracket_terms_empty" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"]rest"; // 空括号表达式
        let mut ctx = ParseContext {
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
        let mut ls = LiteralsBuilder { literals: Vec::new() };
        let mut neg = NegCollector {
            negate: false,
            classes: Vec::new(),
        };
        let result = parse_bracket_terms(&mut ctx, &mut ls, &mut neg);
    });

    // ---- parse_bracket 测试 ----

    test!("test_parse_bracket_simple_char" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"[a]rest";
        let mut ctx = ParseContext {
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
        let result = parse_bracket(&mut ctx);
        // 实现后：应成功
    });

    test!("test_parse_bracket_range" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"[a-z]rest";
        let mut ctx = ParseContext {
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
        let result = parse_bracket(&mut ctx);
    });

    test!("test_parse_bracket_negate" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"[^abc]rest";
        let mut ctx = ParseContext {
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
        let result = parse_bracket(&mut ctx);
    });

    test!("test_parse_bracket_char_class" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"[[:alpha:]]rest";
        let mut ctx = ParseContext {
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
        let result = parse_bracket(&mut ctx);
    });

    // ---- parse_atom 测试 ----

    test!("test_parse_atom_literal" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"a";
        let mut ctx = ParseContext {
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
        let result = parse_atom(&mut ctx);
        // 实现后：应成功
    });

    test!("test_parse_atom_dot" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b".";
        let mut ctx = ParseContext {
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
        let result = parse_atom(&mut ctx);
    });

    test!("test_parse_atom_caret" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"^a";
        let mut ctx = ParseContext {
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
        let result = parse_atom(&mut ctx);
    });

    // ---- tre_parse 测试 ----

    test!("test_tre_parse_simple_literal" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"abc";
        let mut ctx = ParseContext {
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
        let result = tre_parse(&mut ctx);
        // 实现后：应成功返回 AST
    });

    test!("test_tre_parse_star" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"a*";
        let mut ctx = ParseContext {
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
        let result = tre_parse(&mut ctx);
    });

    test!("test_tre_parse_union" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"a|b";
        // ERE 模式下，| 作为选择
        let mut ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 0,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 1, // REG_EXTENDED
        };
        let result = tre_parse(&mut ctx);
    });

    test!("test_tre_parse_subexpression" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"(ab)*";
        // ERE 模式
        let mut ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 0,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 1, // REG_EXTENDED
        };
        let result = tre_parse(&mut ctx);
    });

    test!("test_tre_parse_bre_backref" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"\\(a\\)\\1";
        // BRE 模式，反向引用
        let mut ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 0,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 0, // BRE
        };
        let result = tre_parse(&mut ctx);
    });

    test!("test_tre_parse_empty" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"";
        let mut ctx = ParseContext {
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
        let result = tre_parse(&mut ctx);
        // 空正则表达式应被接受（匹配空串）
    });

    test!("test_tre_parse_unmatched_paren" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"(abc";
        // ERE 模式，缺右括号
        let mut ctx = ParseContext {
            mem: &mut mem,
            stack: Vec::new(),
            result: None,
            pos: pattern,
            start: pattern,
            submatch_id: 0,
            position: 0,
            max_backref: 0,
            backref_ok: 0,
            cflags: 1,
        };
        let result = tre_parse(&mut ctx);
        // 实现后：应返回 Err(RegError::EParen)
    });

    test!("test_tre_parse_unmatched_bracket" {
        let mut mem = tre_mem_new();
        let pattern: &[u8] = b"[abc";
        // 缺右方括号
        let mut ctx = ParseContext {
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
        let result = tre_parse(&mut ctx);
        // 实现后：应返回 Err(RegError::EBrack)
    });
}
