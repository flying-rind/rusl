# regcomp / regfree Rust 接口规约

## 概述

本模块实现 POSIX `regcomp()` 和 `regfree()` 函数。`regcomp()` 将正则表达式字符串编译为内部 TNFA 格式，编译管线包括：解析 → AST 构造 → Tag 注入 → AST 展开 → NFL 计算 → TNFA 生成。Rust 实现中，对外 `regcomp` / `regfree` 签名保持 ABI 兼容，内部整个编译管线可用 Rust 安全抽象、所有权模型和模式匹配重构，大量内部函数和类型可用 Rust 惯用法替代 C 的显式栈和手动内存管理。

---

## 依赖图

```
regcomp (Public)
  ├── tre_parse ─────────────→ 解析器（BRE/ERE 正则 → AST）
  │     ├── parse_atom ──────→ 原子解析
  │     │     ├── parse_bracket → 括号表达式解析
  │     │     ├── tre_expand_macro → 简写展开 (\w, \d 等)
  │     │     └── hexval → 十六进制字符转换
  │     └── parse_dup ───────→ 重复计数解析 {m,n}
  ├── tre_add_tags ──────────→ Tag 注入（子匹配位置标记）
  ├── tre_expand_ast ────────→ AST 迭代展开 ({m,n} → 连接/并集)
  ├── tre_compute_nfl ───────→ Nullable/Firstpos/Lastpos 计算
  ├── tre_ast_to_tnfa ───────→ AST → TNFA 转换
  ├── tre_stack (Internal) ──→ 动态栈（用于非递归遍历）
  └── tre_mem (Internal) ───→ 内存分配器

regfree (Public)
  └── 释放 Tnfa 所有子结构
```

---

## [RELY]

Predefined Structures/Functions:
  `regex_t` / `regmatch_t` / `regoff_t` (type, `<regex.h>`)        // 依赖1: POSIX 公共类型
  `Tnfa` / `TnfaTransition` / `SubmatchData` (struct, tre 模块)     // 依赖2: TNFA 核心数据结构
  `TagDirection` (enum, tre 模块)                                   // 依赖3: tag 匹配方向
  `TreCint` / `TreCtype` (type alias, tre 模块)                     // 依赖4: 宽字符类型
  `ASSERT_*` (const, tre 模块)                                      // 依赖5: 断言位掩码
  `TreMem` (struct, tre_mem 模块)                                   // 依赖6: 内存分配器
  `mbtowc` / `memset` / `memcpy` / `qsort` / `isdigit` (libc)      // 依赖7: 标准库函数
  `iswalnum` / `iswctype` / `towlower` / `towupper` (libc)         // 依赖8: 宽字符函数
  `c_char`, `c_int`, `c_size_t` (std::ffi / libc 类型)              // 依赖9: C ABI 兼容类型
  `RE_DUP_MAX` (const, `<regex.h>`)                                 // 依赖10: 重复次数上限
  `CHARCLASS_NAME_MAX` (const, `<regex.h>`)                         // 依赖11: 字符类名称最大长度

---

## [GUARANTEE]

Exported Interface:

```rust
extern "C" fn regcomp(preg: *mut regex_t, regex: *const c_char, cflags: c_int) -> c_int
extern "C" fn regfree(preg: *mut regex_t)
```

本模块保证对外提供的接口签名，ABI 兼容 POSIX `regcomp()` / `regfree()`。

---

## 第一部分：AST 类型定义

### AstType — AST 节点类型枚举

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) enum AstType {
    Literal,       // 字面量节点（单字符、断言、标签、反向引用、空串）
    Catenation,    // 连接节点
    Iteration,     // 迭代节点 (* + ? {m,n})
    Union,         // 并集节点 (|)
}
```

### LiteralSubtype — 字面量子类型

```rust
// [Visibility]: Internal — rusl crate 内部
// 对应 C 的 EMPTY=-1, ASSERTION=-2, TAG=-3, BACKREF=-4 负值编码
pub(crate) enum LiteralKind {
    Char(i64),         // 普通字符（code_min >= 0）
    Empty,             // 空叶节点
    Assertion(i32),    // 断言叶节点（携带断言类型）
    Tag(i32),          // 标签叶节点（携带 tag_id）
    Backref(i32),      // 反向引用叶节点（携带引用编号）
}
```

**Rust 设计优势**：C 实现通过 `code_min` 的负值区分特殊节点类型。Rust 使用带数据的枚举，类型安全且无需 IS_SPECIAL / IS_EMPTY 等宏。

### AstNode — AST 通用节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct AstNode {
    pub node_type: AstType,
    pub nullable: Option<bool>,              // None = 未计算，Some(false) = 不可空，Some(true) = 可空
    pub submatch_id: Option<u32>,            // 子匹配根节点 ID
    pub num_submatches: u32,
    pub num_tags: u32,
    pub firstpos: Option<Vec<PosAndTags>>,   // None = 未计算
    pub lastpos: Option<Vec<PosAndTags>>,    // None = 未计算
    pub obj: AstNodeObj,
}

pub(crate) enum AstNodeObj {
    Literal(Literal),
    Catenation(Catenation),
    Iteration(Iteration),
    Union(Union),
}
```

### Literal — 字面量节点数据

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct Literal {
    pub kind: LiteralKind,              // 替代 C 的 code_min/code_max 双字段 + 负值编码
    pub position: Option<u32>,          // 在正则表达式中的位置序号
    pub class: Option<TreCtype>,        // 字符类别（如 [:alnum:]）
    pub neg_classes: Option<Vec<TreCtype>>,  // 否定字符类别列表
}
```

### Catenation — 连接节点数据

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct Catenation {
    pub left: Box<AstNode>,   // 左子表达式（除最后一个外的所有）
    pub right: Box<AstNode>,  // 右子表达式（最后一个）
}
```

### Iteration — 迭代节点数据

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct Iteration {
    pub arg: Box<AstNode>,   // 子表达式
    pub min: i32,            // 最小重复次数
    pub max: i32,            // 最大重复次数（-1 表示无上限）
    pub minimal: bool,       // true = 非贪婪匹配
}
```

### Union — 并集节点数据

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct Union {
    pub left: Box<AstNode>,
    pub right: Box<AstNode>,
}
```

### PosAndTags — 位置-标签组合

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct PosAndTags {
    pub position: i32,
    pub code_min: i64,
    pub code_max: i64,
    pub tags: Option<Vec<i32>>,         // 替代 C 的 int *tags
    pub assertions: i32,
    pub class: Option<TreCtype>,
    pub neg_classes: Option<Vec<TreCtype>>,
    pub backref: Option<i32>,
}
```

---

## 第二部分：解析器内部类型

### LiteralsBuilder — 字面量数组构造器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct LiteralsBuilder {
    pub literals: Vec<Literal>,  // 替代 C 的 tre_literal_t **a + len + cap
}
```

**Rust 设计优势**：C 实现手动管理动态数组（`tre_literal_t **a` + `len` + `cap` + `realloc`）。Rust 使用 `Vec<Literal>` 自动管理容量和增长。

### NegCollector — 否定字符类收集器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct NegCollector {
    pub negate: bool,
    pub classes: Vec<TreCtype>,  // 替代 C 的固定 64 元素数组 + len
}
```

### ParseContext — 解析上下文

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct ParseContext<'a> {
    pub mem: &'a mut TreMem,          // TRE 内存分配器
    pub stack: Vec<StackItem>,        // 替代 C 的自定义 tre_stack_t
    pub result: Option<Box<AstNode>>, // 解析结果 AST 根节点
    pub pos: &'a [u8],               // 剩余待解析的字节切片
    pub start: &'a [u8],             // 原始正则表达式
    pub submatch_id: u32,
    pub position: u32,
    pub max_backref: u32,
    pub backref_ok: u32,             // 位掩码
    pub cflags: c_int,
}
```

**Rust 设计优势**：
- C 的 `tre_parse_ctx_t` 使用裸指针追踪解析位置；Rust 使用切片 + 偏移量
- C 的自定义 `tre_stack_t` 被 `Vec<StackItem>` 替代，利用 Rust 标准库
- C 的 `start` / `s` 裸指针被生命周期关联的切片引用替代

---

## 第三部分：AST 节点构造函数

### ast_new_literal — 创建字面量节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn ast_new_literal(mem: &mut TreMem, kind: LiteralKind, position: u32) -> Option<Box<AstNode>>
```

**意图**：创建一个 LITERAL 类型的 AST 节点。

**前置条件**：`mem` 为有效的分配器。

**后置条件**：
- Case 成功：返回 `Some(Box<AstNode>)`，节点类型为 `AstType::Literal`
- Case 失败（内存不足）：`mem.failed == true`，返回 `None`

### ast_new_catenation — 创建连接节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn ast_new_catenation(mem: &mut TreMem, left: Option<Box<AstNode>>, right: Box<AstNode>) -> Option<Box<AstNode>>
```

**意图**：创建 CATENATION 节点。若 `left` 为 `None`，直接返回 `right`（优化空连接）。

### ast_new_iter — 创建迭代节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn ast_new_iter(mem: &mut TreMem, arg: Box<AstNode>, min: i32, max: i32, minimal: bool) -> Option<Box<AstNode>>
```

**意图**：创建 ITERATION 节点包装子表达式。`max = -1` 表示无上限，`minimal = true` 表示非贪婪。

### ast_new_union — 创建并集节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn ast_new_union(mem: &mut TreMem, left: Option<Box<AstNode>>, right: Box<AstNode>) -> Option<Box<AstNode>>
```

**意图**：创建 UNION 节点。若 `left` 为 `None`，直接返回 `right`。

---

## 第四部分：解析器辅助函数

### tre_expand_macro — 正则简写展开

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_expand_macro(ch: u8) -> Option<&'static str>
```

**意图**：将 `\t`、`\n`、`\r`、`\f`、`\a`、`\e`、`\w`、`\W`、`\s`、`\S`、`\d`、`\D` 等简写展开为对应的字符或等价 bracket 表达式。

**展开映射**：
| 输入 | 展开 |
|------|------|
| `\w` | `"[[:alnum:]_]"` |
| `\W` | `"[^[:alnum:]_]"` |
| `\s` | `"[[:space:]]"` |
| `\S` | `"[^[:space:]]"` |
| `\d` | `"[[:digit:]]"` |
| `\D` | `"[^[:digit:]]"` |

### hexval — 十六进制字符值

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn hexval(c: u8) -> Option<u8>  // 返回 0-15，非法返回 None
```

### parse_dup_count — 解析重复计数

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn parse_dup_count(input: &[u8]) -> (Option<i32>, &[u8])
```

**意图**：从 `input` 解析十进制数字串，返回 `(Some(n), remaining)` 或 `(None, input)`。

### add_icase_literals — 大小写折叠字面量扩展

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn add_icase_literals(ls: &mut LiteralsBuilder, min: i64, max: i64) -> Result<(), RegError>
```

**意图**：对于 `REG_ICASE` 模式，将码点范围 `[min, max]` 中的字符取其对应大小写加入字面量集合。

---

## 第五部分：核心解析器

### parse_bracket_terms — 括号表达式项解析

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn parse_bracket_terms(
    ctx: &mut ParseContext,
    ls: &mut LiteralsBuilder,
    neg: &mut NegCollector,
) -> Result<(), RegError>
```

**意图**：解析 `[...]` 或 `[^...]` 内的项序列。支持单字符字面量、字符范围 `a-z`、字符类 `[:alpha:]`。（不支持排序符号 `[.ch.]` 和等价类 `[=ch=]`）

**前置条件**：`ctx.pos` 指向 `[` 或 `[^` 之后。

**后置条件**：
- Case 成功：`ctx.pos` 更新为 `]` 之后，`ls` 包含解析出的字面量，`neg` 包含否定字符类列表
- Case 失败：返回对应 `RegError` 变体

### parse_bracket — 括号表达式完整解析

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn parse_bracket(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError>
```

**意图**：解析 `[...]` 或 `[^...]`，构建 UNION 树。

**系统算法**：
1. 调用 `parse_bracket_terms` 收集字面量
2. 若为否定（`[^...]`）：排序字面量 + 计算补集 + 收集否定字符类
3. 将所有字面量构建为 UNION 树
4. `ctx.position += 1`

### parse_atom — 正则表达式原子解析

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn parse_atom(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError>
```

**意图**：解析一个正则表达式原子（最小匹配单元）。支持：
- 括号表达式 `[...]`
- 转义序列 `\t`、`\n`、`\xHH`、`\x{HHHH}`、`\b`、`\B`
- 反向引用 `\1`..`\9` (BRE)
- 字面量字符、`.` 通配符
- `^` 行首断言、`$` 行尾断言
- BRE 扩展：`\|` 选择、`\+`、`\?` 重复

**REG_ICASE 处理**：对字面量字符，创建并行的 `towupper`/`towlower` UNION 节点。

### tre_parse — 正则表达式顶层解析器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_parse(ctx: &mut ParseContext) -> Result<Box<AstNode>, RegError>
```

**意图**：完整解析 BRE 或 ERE 正则表达式，构建 AST 树。支持嵌套子表达式 `\(...\)` / `(...)` 和选择 `\|` / `|`。

**系统算法**（基于显式栈的迭代式解析）：
1. 栈初始化：第一层子匹配 ID=0 入栈
2. 主循环：
   - 遇到 `\(`(BRE) 或 `(`(ERE)：新子表达式开始 → 入栈
   - 断言/空标记字符串结束 → 生成空字面量
   - 否则：调用 `parse_atom` 解析原子
   - 检查后续重复运算符 `*`/`+`/`?`/`{...}` → 包装为 ITERATION 节点
   - 将原子/迭代节点拼接到当前 Branch
   - 遇到 `|` 或 `\)`/`)` 或结束 → 将当前 Branch 加入并集
   - 子表达式结束时 → 调用 `marksub` 标记 → 出栈

**语法支持（musl 扩展）**：
- BRE 中 `\|` 作为选择、`\+`/`\?` 作为重复
- 空分支（如 `()`、`(a|)`）被接受但匹配空串

**前置条件**：`ctx` 已正确初始化。

**后置条件**：
- Case 成功：`ctx.result` 指向完整的 AST 根节点
- Case 失败：返回对应 `RegError` 变体（`EPAREN`、`BADBR`、`ESPACE` 等）

---

## 第六部分：Tag 注入

### marksub — 子匹配根节点标记

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn marksub(ctx: &mut ParseContext, node: &mut AstNode, subid: u32) -> Result<(), RegError>
```

**意图**：将 `node` 标记为 `subid` 子匹配的根节点。

### add_tag_left / add_tag_right — 插入标签节点

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn add_tag_left(mem: &mut TreMem, node: &mut AstNode, tag_id: i32) -> Result<(), RegError>
pub(crate) fn add_tag_right(mem: &mut TreMem, node: &mut AstNode, tag_id: i32) -> Result<(), RegError>
```

**意图**：在节点左侧/右侧插入 TAG 字面量。原节点类型被替换为 CATENATION。

### tre_add_tags — 子匹配标签注入

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_add_tags(
    mem: &mut TreMem,
    stack: &mut Vec<StackItem>,
    tree: &mut AstNode,
    tnfa: &mut TnfaBuilder
) -> Result<(), RegError>
```

**意图**：两遍遍历 AST 树，为子匹配表达式插入标签节点（TAG literal）。

**系统算法**（两遍）：
- **第一遍** (`tnfa` 为 `None`)：计算每个 AST 节点的 `num_tags`（需要的标签数）
- **第二遍**：为每个需要标记的位置插入 TAG 字面量，填充 `TnfaBuilder` 的 `tag_directions`、`minimal_tags`、`submatch_data` 等

---

## 第七部分：AST 变换

### tre_copy_ast — AST 子树深拷贝

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_copy_ast(
    mem: &mut TreMem,
    ast: &AstNode,
    flags: CopyFlags,
    pos_add: &mut u32,
    tag_directions: &[TagDirection],
    max_pos: &mut u32,
) -> Result<Box<AstNode>, RegError>
```

**意图**：复制 AST 子树，支持标签移除和首个标签最大化模式。Rust 实现中基于递归而非显式栈（安全且简洁）。

### tre_expand_ast — 迭代节点展开

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_expand_ast(
    mem: &mut TreMem,
    ast: &mut AstNode,
    position: &mut u32,
    tag_directions: &[TagDirection],
) -> Result<(), RegError>
```

**意图**：将 `{m,n}` 迭代节点展开为可能的匹配序列。例如 `a{3}` → `aaa`，`a{2,4}` → `aa(a(a|)|)`。

**系统算法**：
- 对每个 ITERATION 节点：
  - 若 `min > 1` 或 `max > 1`：创建 `min` 个副本的连接；展开剩余可选部分
  - 将展开结果合并回原节点位置
  - 更新所有 LITERAL 的 `position` 域

---

## 第八部分：NFL 计算

### tre_set_empty / tre_set_one / tre_set_union — 位置集合操作

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_set_empty() -> Vec<PosAndTags>
pub(crate) fn tre_set_one(mem: &mut TreMem, pos: i32, code_min: i64, code_max: i64, ...) -> Vec<PosAndTags>
pub(crate) fn tre_set_union(mem: &mut TreMem, set1: &[PosAndTags], set2: &[PosAndTags], ...) -> Vec<PosAndTags>
```

### tre_match_empty — 可空路径计算

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_match_empty(
    node: &AstNode,
    tags: &mut Vec<i32>,
    assertions: &mut i32,
    num_tags_seen: &mut u32,
) -> Result<(), RegError>
```

**意图**：遍历 AST 寻找可匹配空串的路径，收集路径上的 TAG 和 ASSERTION。

### tre_compute_nfl — Nullable/Firstpos/Lastpos 计算

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_compute_nfl(mem: &mut TreMem, tree: &mut AstNode) -> Result<(), RegError>
```

**意图**：对 AST 每个节点计算 `nullable`、`firstpos`、`lastpos` 属性。

**语义定义**：
- **nullable**：该子树能否匹配空串
- **firstpos**：该子树匹配的第一个字符可能来自的位置集合
- **lastpos**：该子树匹配的最后一个字符可能来自的位置集合

**系统算法**（自底向上）：

| 节点类型 | nullable | firstpos | lastpos |
|---------|----------|----------|---------|
| LITERAL | false | {position} | {position} |
| TAG/ASSERTION/EMPTY | true | {} | {} |
| UNION | left \|\| right | left ∪ right | left ∪ right |
| ITERATION | min==0 \|\| arg.nullable | arg.firstpos | arg.lastpos |
| CATENATION | left && right | left.nullable ? ... | right.nullable ? ... |

---

## 第九部分：TNFA 构建

### tre_make_trans — 创建位置间转移

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_make_trans(
    p1: &[PosAndTags],
    p2: &[PosAndTags],
    transitions: &mut Vec<TnfaTransition>,
    counts: &mut [u32],
    offs: &mut [u32],
) -> Result<(), RegError>
```

**意图**：从 `p1` 每个位置向 `p2` 每个位置创建转移边。采用两遍设计：先统计出边数量，再填充转移表。

### tre_ast_to_tnfa — AST 到 TNFA 编译

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_ast_to_tnfa(
    node: &AstNode,
    tnfa: &mut TnfaBuilder,
) -> Result<(), RegError>
```

**意图**：递归遍历 AST，为每个 CATENATION 和 ITERATION 节点创建转移边。

**系统算法**：
- LITERAL：无操作（转移由父节点创建）
- UNION：递归处理左右子
- CATENATION：连接 `left.lastpos` 到 `right.firstpos`，然后递归处理
- ITERATION（max == -1）：连接 `arg.lastpos` 到 `arg.firstpos`（形成循环），然后递归处理

---

## 第十部分：regcomp (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn regcomp(
    preg: *mut regex_t,
    regex: *const c_char,
    cflags: c_int,
) -> c_int
```

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。

### 意图 (Intent)

将正则表达式字符串 `regex` 编译为内部 TNFA 格式，存入 `preg`。编译结果供 `regexec` 使用。

### 编译管线 (Level 1)

```
输入: regex 字符串 + cflags
  1. 创建 TreMem 分配器和 Vec 栈
  2. 解析 (tre_parse) → AST 树
  3. 校验反向引用不越界 (max_backref <= re_nsub)
  4. 标签注入 (tre_add_tags, 两遍) → 子匹配位置标记
  5. 迭代展开 (tre_expand_ast) → {m,n} → 连接/并集
  6. NFL 计算 (tre_compute_nfl) → nullable/firstpos/lastpos
  7. TNFA 转移统计 (tre_ast_to_tnfa, 第一遍)
  8. TNFA 转移填充 (tre_ast_to_tnfa, 第二遍)
  9. 初始状态转移表构建
  10. 清理临时资源，Tnfa 存入 preg.__opaque
输出: preg（已编译正则表达式）
```

### 前置条件

- `preg != NULL`：`regex_t` 指针有效
- `regex != NULL`：指向以 `\0` 结尾的正则表达式字符串
- `cflags` 由以下标志按位或构成：
  - `REG_EXTENDED`：使用 ERE 语法，否则 BRE
  - `REG_ICASE`：大小写不敏感匹配
  - `REG_NOSUB`：不需要子匹配信息（跳过标签注入）
  - `REG_NEWLINE`：特殊对待换行符

### 后置条件

| 条件 | 返回值 | `preg` 状态 |
|------|--------|-------------|
| 编译成功 | `REG_OK` (0) | `re_nsub` 设为子表达式数量，`__opaque` 指向编译好的 `Tnfa` |
| 编译失败 | 非零错误码 | `__opaque` 可能为非 NULL（部分构造），需 `regfree` 释放 |

### 错误码

| 错误码 | 值 | 含义 |
|--------|-----|------|
| `REG_OK` | 0 | 成功 |
| `REG_BADPAT` | 2 | 正则表达式语法错误 |
| `REG_ECOLLATE` | 3 | 无效排序元素 |
| `REG_ECTYPE` | 4 | 无效字符类名 |
| `REG_EESCAPE` | 5 | 尾部转义符 |
| `REG_ESUBREG` | 6 | 引用不存在的子表达式 |
| `REG_EBRACK` | 7 | 括号不匹配（缺 `]`） |
| `REG_EPAREN` | 8 | 括号不匹配（缺 `)`） |
| `REG_EBRACE` | 9 | 花括号不匹配 |
| `REG_BADBR` | 10 | 非法 `\{m,n\}` 语法 |
| `REG_ERANGE` | 11 | 非法字符范围 |
| `REG_ESPACE` | 12 | 内存不足 |
| `REG_BADRPT` | 13 | 非法重复运算符 |

### 不变量 (Invariants)

- 编译过程不修改 `regex` 字符串内容
- 若编译成功返回，`preg` 可用于 `regexec`
- 若编译失败，调用者应调用 `regfree(preg)` 释放
- Rust 实现中，若编译失败，通过 RAII 自动清理已分配资源

---

## 第十一部分：regfree (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn regfree(preg: *mut regex_t)
```

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。

### 意图 (Intent)

释放 `regcomp` 编译产生的所有内存资源。调用后 `preg` 可被重新用于下一次 `regcomp` 或丢弃。

Rust 内部实现中，由于 `Tnfa` 的所有子字段使用 `Box<[T]>` / `Vec<T>` 等 RAII 类型，释放逻辑由 `Drop` trait 自动实现。`regfree` 的实现简化为：

```
1. 若 preg.__opaque != null:
    a. 将 __opaque 转换为 Box<Tnfa>
    b. drop(Box<Tnfa>)  // 自动递归释放所有子结构
    c. preg.__opaque = null
2. 重置 re_nsub = 0
```

### 前置条件

- `preg != NULL`
- `preg.__opaque` 要么为 NULL（未编译或已释放），要么指向有效的 `Tnfa`

### 后置条件

- `preg` 不再持有任何动态分配的资源
- 多次调用 `regfree(preg)` 是安全的（NULL 检查）

---

## 安全与正确性属性

1. **所有权模型消除内存泄漏**：Rust 的 `Box`/`Vec` 层次自动管理 AST 节点和 TNFA 的内存。编译失败回滚路径中，已分配资源通过 RAII 自动释放，无需 C 的 `error_exit` goto 标签。

2. **枚举替代负值编码**：`LiteralKind` 枚举替代 C 的 `code_min < 0` 负值编码（EMPTY/ASSERTION/TAG/BACKREF），消除 IS_SPECIAL / IS_EMPTY 等宏的类型不安全。

3. **Vec 替代手动栈和动态数组**：C 的 `tre_stack_t`（手动 realloc 的动态数组）和 `struct literals`（手动管理的字面量数组）被 `Vec<T>` 替代，堆管理和增长策略由标准库负责。

4. **显式栈保留非递归遍历**：C 中使用显式栈的函数（`tre_parse`、`tre_add_tags`、`tre_copy_ast`、`tre_expand_ast`、`tre_compute_nfl`、`tre_match_empty`）在 Rust 中可以：
   - 深度有限的 AST：使用递归 + `Result` 的错误传播
   - 深度不可预期：保留显式栈但使用 `Vec<StackItem>` 而非 C 的自定义栈

5. **POSIX 左优先语义**：UNION 节点保证左侧优先于右侧，与 C 实现一致。

---

## 引用关系

| 符号 | 可见性 | 被引用者 |
|------|--------|----------|
| `regcomp` | Public | `<regex.h>` |
| `regfree` | Public | `<regex.h>` |
| `AstNode` / `AstType` / `LiteralKind` | Internal | 整个编译管线 |
| `ParseContext` / `LiteralsBuilder` | Internal | 解析器 |
| `tre_parse` / `parse_atom` / `parse_bracket` | Internal | `regcomp` 编译管线 |
| `tre_add_tags` | Internal | `regcomp` 编译管线 |
| `tre_expand_ast` / `tre_copy_ast` | Internal | `regcomp` 编译管线 |
| `tre_compute_nfl` | Internal | `regcomp` 编译管线 |
| `tre_ast_to_tnfa` / `tre_make_trans` | Internal | `regcomp` 编译管线 |
| `marksub` / `add_tag_left` / `add_tag_right` | Internal | Tag 注入阶段 |
| `tre_expand_macro` / `hexval` | Internal | 解析器辅助 |
