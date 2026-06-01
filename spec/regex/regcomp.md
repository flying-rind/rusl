# regcomp.c 规约

> TRE (TRE Regular Expression) 正则表达式编译模块，基于 Ville Laurikari 的 TRE 库实现。
> 本文件包含正则表达式的完整编译管线：解析 → AST 构造 → Tag 注入 → AST 展开 → NFL 计算 → TNFA 生成。

---

## 依赖图

```
regcomp
  ├── tre_parse
  │     ├── parse_atom
  │     │     ├── parse_bracket
  │     │     │     ├── parse_bracket_terms
  │     │     │     │     ├── tre_new_lit
  │     │     │     │     └── add_icase_literals
  │     │     │     ├── tre_new_lit
  │     │     │     ├── tre_compare_lit (qsort 比较器)
  │     │     │     └── add_icase_literals
  │     │     ├── tre_expand_macro
  │     │     └── hexval
  │     ├── parse_dup → parse_dup_count
  │     └── marksub
  ├── tre_add_tags → tre_add_tag_left, tre_add_tag_right, tre_purge_regset
  ├── tre_expand_ast → tre_copy_ast
  ├── tre_compute_nfl → tre_match_empty, tre_set_one, tre_set_empty, tre_set_union
  ├── tre_ast_to_tnfa → tre_make_trans
  ├── tre_mem_new / tre_mem_destroy (see tre-mem.c spec)
  ├── tre_stack_new / tre_stack_destroy (内部栈)
  └── regfree (回滚时调用)

regfree
  └── (直接释放 TNFA 所有子结构)
```

---

## 一、内部类型定义

### tre_ast_type_t（AST 节点类型枚举）

```c
typedef enum { LITERAL, CATENATION, ITERATION, UNION } tre_ast_type_t;
```

[Visibility]: Internal — musl TRE 内部 AST 表示，POSIX/ISO C 标准未定义

定义 AST 节点的四种基本类型：
- `LITERAL`：字面量节点（单字符、断言、标签、反向引用、空串）
- `CATENATION`：连接节点，表示两个子表达式的顺序拼接
- `ITERATION`：迭代节点，表示 `*`、`+`、`?`、`{m,n}` 重复
- `UNION`：并集节点，表示 `|` 选择

### 字面量子类型宏

```c
#define EMPTY      -1   // 空叶节点（表示空字符串）
#define ASSERTION  -2   // 断言叶节点
#define TAG        -3   // 标签叶节点
#define BACKREF    -4   // 反向引用叶节点
```

[Visibility]: Internal — AST 实现细节，通过 `code_min` 的负值编码特殊节点类型

### tre_ast_node_t（AST 通用节点）

```c
typedef struct {
    tre_ast_type_t type;
    void *obj;
    int nullable;
    int submatch_id;
    int num_submatches;
    int num_tags;
    tre_pos_and_tags_t *firstpos;
    tre_pos_and_tags_t *lastpos;
} tre_ast_node_t;
```

[Visibility]: Internal — TRE AST 节点结构

**字段语义**：
- `type`：节点类型
- `obj`：指向具体节点数据（`tre_literal_t`/`tre_catenation_t`/`tre_iteration_t`/`tre_union_t`）
- `nullable`：该子树能否匹配空串（-1 未计算，0 不可，1 可）
- `submatch_id`：若此节点是某个子匹配的根，记录子匹配 ID（-1 表示非子匹配根）
- `num_submatches`：该子树内子匹配总数
- `num_tags`：该子树内标签总数
- `firstpos`：该子树第一个位置集合
- `lastpos`：该子树最后一个位置集合

### tre_literal_t（字面量节点）

```c
typedef struct {
    long code_min;
    long code_max;
    int position;
    tre_ctype_t class;
    tre_ctype_t *neg_classes;
} tre_literal_t;
```

[Visibility]: Internal — 表示单个字面量匹配

**字段语义**：
- `code_min` / `code_max`：匹配的字符码点范围。若 `code_min < 0`，则为特殊节点（EMPTY/ASSERTION/TAG/BACKREF）
- `position`：在正则表达式模式中的位置序号
- `class`：字符类（如 `[:alnum:]`），0 表示无
- `neg_classes`：否定字符类数组，NULL 表示无

### tre_catenation_t（连接节点）

```c
typedef struct {
    tre_ast_node_t *left;
    tre_ast_node_t *right;
} tre_catenation_t;
```

[Visibility]: Internal — 连接是左结合的，`left` 持有除最后一个外的所有子表达式，`right` 持有最后一个

### tre_iteration_t（迭代节点）

```c
typedef struct {
    tre_ast_node_t *arg;
    int min;
    int max;
    unsigned int minimal:1;
} tre_iteration_t;
```

[Visibility]: Internal — 表示 `*`(min=0,max=-1)、`+`(min=1,max=-1)、`?`(min=0,max=1)、`{m,n}` 重复

`max = -1` 表示无上限。`minimal = 1` 表示非贪婪匹配。

### tre_union_t（并集节点）

```c
typedef struct {
    tre_ast_node_t *left;
    tre_ast_node_t *right;
} tre_union_t;
```

[Visibility]: Internal — 表示 `|` 选择

### tre_pos_and_tags_t（位置-标签组合）

```c
typedef struct {
    int position;
    int code_min;
    int code_max;
    int *tags;
    int assertions;
    tre_ctype_t class;
    tre_ctype_t *neg_classes;
    int backref;
} tre_pos_and_tags_t;
```

[Visibility]: Internal — 用于 NFL (nullable/firstpos/lastpos) 计算的位置集合

位置集合以 `position = -1` 的元素终止。

### tre_stack_rec / tre_stack_t（内部栈）

```c
typedef struct tre_stack_rec {
    int size;
    int max_size;
    int increment;
    int ptr;
    union tre_stack_item *stack;
} tre_stack_t;

union tre_stack_item {
    void *voidptr_value;
    int int_value;
};
```

[Visibility]: Internal — musl TRE 内部使用的动态数组栈，非 POSIX 标准接口

支持 `int` 和 `void *` 两种类型的推入/弹出。栈满时自动 `realloc` 增长至 `max_size`。

### tre_parse_ctx_t（解析上下文）

```c
typedef struct {
    tre_mem_t mem;
    tre_stack_t *stack;
    tre_ast_node_t *n;
    const char *s;
    const char *start;
    int submatch_id;
    int position;
    int max_backref;
    int backref_ok;
    int cflags;
} tre_parse_ctx_t;
```

[Visibility]: Internal — 解析器状态上下文

### struct literals（字面量数组构造器）

```c
struct literals {
    tre_mem_t mem;
    tre_literal_t **a;
    int len;
    int cap;
};
```

[Visibility]: Internal — 用于 bracket 表达式解析时收集字面量

### struct neg（否定字符类收集器）

```c
struct neg {
    int negate;
    int len;
    tre_ctype_t a[64];
};
```

[Visibility]: Internal — 用于 `[^...]` 否定括号表达式中的字符类收集

### tre_addtags_symbol_t（标签添加遍历状态）

```c
typedef enum {
    ADDTAGS_RECURSE,
    ADDTAGS_AFTER_ITERATION,
    ADDTAGS_AFTER_UNION_LEFT,
    ADDTAGS_AFTER_UNION_RIGHT,
    ADDTAGS_AFTER_CAT_LEFT,
    ADDTAGS_AFTER_CAT_RIGHT,
    ADDTAGS_SET_SUBMATCH_END
} tre_addtags_symbol_t;
```

[Visibility]: Internal — 模拟递归栈的状态枚举

### tre_copyast_symbol_t / tre_expand_ast_symbol_t / tre_nfl_stack_symbol_t

[Visibility]: Internal — 各 AST 遍历函数的状态枚举，用于基于显式栈的迭代式遍历

---

## 二、内部辅助函数（AST 节点构造函数族）

### tre_ast_new_node

```c
static tre_ast_node_t *
tre_ast_new_node(tre_mem_t mem, int type, void *obj)
```

[Visibility]: Internal — AST 通用节点分配器

**意图**：分配并初始化一个 `tre_ast_node_t`，设置 `type` 和 `obj`，初始化 `nullable = -1`、`submatch_id = -1`。

**前置条件**：
- `mem` 不为 NULL（有效的 tre_mem_t 分配器）
- `obj` 不为 NULL（实际的节点内容对象）

**后置条件**：
- Case 成功：返回新分配的节点指针，`node->type == type`，`node->obj == obj`，`node->nullable == -1`，`node->submatch_id == -1`
- Case 失败（分配失败或 obj 为 NULL）：返回 0 (NULL)

### tre_ast_new_literal

```c
static tre_ast_node_t *
tre_ast_new_literal(tre_mem_t mem, int code_min, int code_max, int position)
```

[Visibility]: Internal — 字面量 AST 节点构造函数

**意图**：创建一个 LITERAL 类型的 AST 节点，包含指定的码点范围和位置。

**前置条件**：
- `mem` 不为 NULL
- `code_min` 和 `code_max` 为合法码点值（或特殊子类型负值）

**后置条件**：
- Case 成功：返回 LITERAL 节点，`lit->code_min == code_min`，`lit->code_max == code_max`，`lit->position == position`
- Case 失败（内存不足）：返回 NULL

### tre_ast_new_iter

```c
static tre_ast_node_t *
tre_ast_new_iter(tre_mem_t mem, tre_ast_node_t *arg, int min, int max, int minimal)
```

[Visibility]: Internal — 迭代 AST 节点构造函数

**意图**：创建一个 ITERATION 类型节点包装子表达式 `arg`，指定最小/最大重复次数和贪婪模式。

**前置条件**：
- `mem` 不为 NULL
- `arg` 不为 NULL

**后置条件**：
- Case 成功：返回 ITERATION 节点，`iter->arg == arg`，`iter->min == min`，`iter->max == max`，`iter->minimal == minimal`，`node->num_submatches == arg->num_submatches`
- Case 失败：返回 NULL

### tre_ast_new_union

```c
static tre_ast_node_t *
tre_ast_new_union(tre_mem_t mem, tre_ast_node_t *left, tre_ast_node_t *right)
```

[Visibility]: Internal — 并集 AST 节点构造函数

**意图**：创建 UNION 节点。若 `left` 为 NULL，直接返回 `right`（优化空并集）。

**前置条件**：
- `mem` 不为 NULL
- `right` 不为 NULL

**后置条件**：
- Case `left == NULL`：直接返回 `right`（无分配）
- Case 成功：返回 UNION 节点，`left + right` 子匹配总数 = `left->num_submatches + right->num_submatches`
- Case 失败：返回 NULL

### tre_ast_new_catenation

```c
static tre_ast_node_t *
tre_ast_new_catenation(tre_mem_t mem, tre_ast_node_t *left, tre_ast_node_t *right)
```

[Visibility]: Internal — 连接 AST 节点构造函数

**意图**：创建 CATENATION 节点。若 `left` 为 NULL，直接返回 `right`（优化空连接）。

**前置条件**：
- `mem` 不为 NULL

**后置条件**：
- Case `left == NULL`：直接返回 `right`
- Case 成功：返回 CATENATION 节点，`left + right` 子匹配总数为两者之和
- Case 失败：返回 NULL

---

## 三、内部辅助函数（栈操作）

### tre_stack_new

```c
static tre_stack_t *
tre_stack_new(int size, int max_size, int increment)
```

[Visibility]: Internal — musl TRE 内部动态栈

**意图**：分配新栈对象。初始容量 `size`，最大容量 `max_size`，每次增长 `increment`。

**前置条件**：
- `size > 0`
- `max_size >= size`
- `increment > 0`

**后置条件**：
- Case 成功：返回栈对象，`s->size == size`，`s->ptr == 0`
- Case 失败（内存不足）：返回 NULL

### tre_stack_destroy

```c
static void tre_stack_destroy(tre_stack_t *s)
```

[Visibility]: Internal

**意图**：释放栈对象及其内部数组。

**前置条件**：`s` 是通过 `tre_stack_new` 分配的有效栈

**后置条件**：`s` 指向的内存被释放

### tre_stack_num_objects

```c
static int tre_stack_num_objects(tre_stack_t *s)
```

[Visibility]: Internal

**意图**：返回栈中当前元素数量。

### tre_stack_push

```c
static reg_errcode_t tre_stack_push(tre_stack_t *s, union tre_stack_item value)
```

[Visibility]: Internal — 通用栈推入操作

**意图**：将 `value` 推入栈顶。若栈满，尝试 `realloc` 扩展至 `max_size` 以内。

**前置条件**：`s` 不为 NULL

**后置条件**：
- Case 成功：返回 `REG_OK`，元素在栈顶
- Case 超出 `max_size` 仍满：返回 `REG_ESPACE`
- Case realloc 失败：返回 `REG_ESPACE`

### tre_stack_push_int / tre_stack_push_voidptr

```c
static reg_errcode_t tre_stack_push_int(tre_stack_t *s, int value)
static reg_errcode_t tre_stack_push_voidptr(tre_stack_t *s, void *value)
```

[Visibility]: Internal — 类型安全的栈推入包装

### tre_stack_pop_int / tre_stack_pop_voidptr

```c
static int tre_stack_pop_int(tre_stack_t *s)
static void *tre_stack_pop_voidptr(tre_stack_t *s)
```

[Visibility]: Internal — 类型安全的栈弹出

**前置条件**：栈非空

**后置条件**：弹出并返回栈顶元素

---

## 四、内部辅助函数（宏展开与比较）

### tre_expand_macro

```c
static const char *tre_expand_macro(const char *s)
```

[Visibility]: Internal — 正则表达式简写展开

**意图**：将 `\t`、`\n`、`\r`、`\f`、`\a`、`\e`、`\w`、`\W`、`\s`、`\S`、`\d`、`\D` 等简写展开为对应的字符或等价 bracket 表达式。

**前置条件**：`s` 指向正则表达式中的一个字符

**后置条件**：
- Case 匹配：返回对应展开字符串（如 `\w` → `"[[:alnum:]_]"`）
- Case 不匹配：返回 NULL (0)

**展开映射**：
| 输入 | 展开 |
|------|------|
| `\t` | `"\t"` |
| `\n` | `"\n"` |
| `\r` | `"\r"` |
| `\f` | `"\f"` |
| `\a` | `"\a"` |
| `\e` | `"\033"` |
| `\w` | `"[[:alnum:]_]"` |
| `\W` | `"[^[:alnum:]_]"` |
| `\s` | `"[[:space:]]"` |
| `\S` | `"[^[:space:]]"` |
| `\d` | `"[[:digit:]]"` |
| `\D` | `"[^[:digit:]]"` |

### tre_compare_lit

```c
static int tre_compare_lit(const void *a, const void *b)
```

[Visibility]: Internal — qsort 比较回调

**意图**：按 `code_min` 升序比较两个字面量节点指针，用于 `qsort` 排序。

**前置条件**：`a` 和 `b` 是指向 `tre_literal_t *` 的指针的指针

**后置条件**：返回负/0/正值表示大小关系

### hexval

```c
static int hexval(unsigned c)
```

[Visibility]: Internal

**意图**：将单个十六进制字符转换为数值（0-15），非法字符返回 -1。

**前置条件**：`c` 为字符码点

**后置条件**：
- Case 有效十六进制字符（0-9, a-f, A-F）：返回 0-15
- Case 非法字符：返回 -1

---

## 五、内部辅助函数（字面量管理）

### tre_new_lit

```c
static tre_literal_t *tre_new_lit(struct literals *p)
```

[Visibility]: Internal — bracket 字面量集合构造器

**意图**：在 `struct literals` 中分配一个新字面量槽位。若容量不足（`len >= cap`），以二倍扩展（上限 `1<<15`）。

**前置条件**：`p` 不为 NULL

**后置条件**：
- Case 成功：`p->len` 递增 1，返回新分配的字面量指针
- Case cap 已达 `1<<15`：返回 NULL (0)
- Case realloc 失败：返回 NULL (0)

### add_icase_literals

```c
static int add_icase_literals(struct literals *ls, int min, int max)
```

[Visibility]: Internal — 大小写折叠字面量扩展

**意图**：对于 `REG_ICASE` 模式，将码点范围 `[min, max]` 中的字符取其对应大小写加入字面量集合。当前不支持多字符对应（如德语的 ß→SS）。

**系统算法**：遍历范围 `[min, max]`，对每个字符判断大小写属性：
- 若为小写 → 取对应大写字符组成范围
- 若为大写 → 取对应小写字符组成范围
- 否则跳过

**前置条件**：
- `ls` 不为 NULL
- `min <= max`

**后置条件**：
- Case 成功：返回 0，大小写对应的字符区间已添加到 `ls` 中
- Case 失败（内存不足）：返回 -1

---

## 六、内部辅助函数（Bracket 表达式解析）

### parse_bracket_terms

```c
static reg_errcode_t parse_bracket_terms(tre_parse_ctx_t *ctx, const char *s,
                                          struct literals *ls, struct neg *neg)
```

[Visibility]: Internal — POSIX bracket 表达式项解析器

**意图**：解析 `[...]` 或 `[^...]` 内的项序列。支持：
- 单字符字面量
- 字符范围 `a-z`
- 字符类 `[:alpha:]`
- (不支持) 排序符号 `[.ch.]` 和等价类 `[=ch=]`

**前置条件**：
- `ctx`、`s`、`ls`、`neg` 均不为 NULL
- `s` 指向 `[` 或 `[^` 之后

**后置条件**：
- Case 成功 (`REG_OK`)：`ctx->s` 更新为 `]` 之后位置，`ls` 包含解析出的字面量，`neg` 包含否定字符类列表
- Case 遇到 `[-...]` 范围错误：返回 `REG_ERANGE`
- Case 遇到 `[.` 或 `[=`（不支持）：返回 `REG_ECOLLATE`
- Case 遇到 `[:name:]` 未知类名：返回 `REG_ECTYPE`
- Case 缺 `]`：返回 `REG_EBRACK`
- Case 非法字节序列：返回 `REG_BADPAT`
- Case 内存不足：返回 `REG_ESPACE`

**不变量**：
- `neg->len <= MAX_NEG_CLASSES (64)`
- `ls->len < ls->cap`

**系统算法**：逐字符扫描，遇到 `]` 终止（除非是开头）；遇到 `-` 解析范围；遇到 `[:` 解析字符类；REG_ICASE 时自动展开大小写。

### parse_bracket

```c
static reg_errcode_t parse_bracket(tre_parse_ctx_t *ctx, const char *s)
```

[Visibility]: Internal — 括号表达式完整解析器

**意图**：解析 `[...]` 或 `[^...]`，构建 UNION 树。

**系统算法**：
1. 调用 `parse_bracket_terms` 收集字面量
2. 若为否定（`[^...]`）：
   - 若 `REG_NEWLINE` 设置，显式排除换行符
   - 用 `qsort` 排序字面量数组
   - 计算每个范围的补集（取相邻字面量范围之间的区间）
   - 收集否定字符类
3. 将所有字面量构建为 UNION 树
4. 释放临时数组，`ctx->position++`

**前置条件**：
- `ctx` 不为 NULL
- `s` 指向 `[` 之后（可能以 `^` 开头）

**后置条件**：
- Case 成功：`ctx->n` 指向 UNION 树，`ctx->position` 递增
- Case 失败：返回相应错误码 (`REG_ESPACE`, `REG_ERANGE`, `REG_EBRACK`, `REG_ECOLLATE`, `REG_ECTYPE`)，`ls.a` 被释放

---

## 七、内部辅助函数（重复计数解析）

### parse_dup_count

```c
static const char *parse_dup_count(const char *s, int *n)
```

[Visibility]: Internal — 解析重复次数字面量

**意图**：从 `s` 解析十进制数字串，结果存入 `*n`。若首字符非数字，设 `*n = -1` 并不推进。

**前置条件**：`s` 和 `n` 不为 NULL

**后置条件**：
- `*n` = 十进制数值（或 `-1` 表示无数字）
- 返回值 = 数字序列后的下一个字符位置（或 `s` 本身若首字符非数字）
- 若数值超过 `RE_DUP_MAX`，停止累加但继续推进指针

### parse_dup

```c
static const char *parse_dup(const char *s, int ere, int *pmin, int *pmax)
```

[Visibility]: Internal — 解析完整重复运算符

**意图**：解析 `\{m,n\}`（BRE）或 `{m,n}`（ERE）形式的重复运算符。

**系统算法**：
1. 解析 `m`（至少数字部分）
2. 若 `,` 存在，解析 `n`；否则 `n = m`
3. 验证语法和范围合法性

**前置条件**：`s`、`pmin`、`pmax` 不为 NULL

**后置条件**：
- Case 成功：`*pmin = m`，`*pmax = n`（-1 表示无上限），返回值指向 `}` 之后
- Case 失败（语法错误、范围不合法）：返回 NULL (0)

**校验规则**：
- `max >= min` 或 `max == -1`（无上限）
- `min >= 0`
- `max <= RE_DUP_MAX`，`min <= RE_DUP_MAX`
- BRE 模式下 `{` 前须有 `\`
- 末尾必须是 `}`

---

## 八、内部辅助函数（子匹配标记）

### marksub

```c
static reg_errcode_t marksub(tre_parse_ctx_t *ctx, tre_ast_node_t *node, int subid)
```

[Visibility]: Internal — 子匹配根节点标记

**意图**：将 `node` 标记为 `subid` 子匹配的根节点。若节点已有子匹配 ID（嵌套子匹配），则在其前插入空字面量并重新创建连接。

**前置条件**：
- `ctx`、`node` 不为 NULL
- `subid >= 0`

**后置条件**：
- Case 成功：`ctx->n` 返回标记后节点，`node->submatch_id == subid`，`node->num_submatches` 递增
- 若 `subid < 10`，`ctx->backref_ok` 的对应位置位
- Case 失败（内存不足）：返回 `REG_ESPACE`

---

## 九、内部辅助函数（核心解析器）

### parse_atom

```c
static reg_errcode_t parse_atom(tre_parse_ctx_t *ctx, const char *s)
```

[Visibility]: Internal — 正则表达式原子解析器

**意图**：解析一个正则表达式原子（最小匹配单元），支持：
- 括号表达式 `[...]`
- 转义序列 `\t`、`\n`、`\xHH`、`\x{HHHH}`、`\b`、`\B`、`\<`、`\>`
- 反向引用 `\1`..`\9` (BRE)
- 字面量字符
- `.` 通配符
- `^` 行首断言、`$` 行尾断言
- BRE 扩展：`\|` 选择、`\+`、`\?` 重复

**前置条件**：
- `ctx` 不为 NULL
- `s` 指向待解析的位置

**后置条件**：
- Case 成功：`ctx->n` 指向解析出的 AST 子树，`ctx->s` 指向下一个待解析位置
- Case 遇到非法转义：返回 `REG_EESCAPE`
- Case 遇到 `\x{HHHH}` 缺 `}`：返回 `REG_EBRACE`
- Case 引用不存在的子表达式：返回 `REG_ESUBREG`
- Case ERE 下空原子后跟重复符：返回 `REG_BADRPT`
- Case 非法字符：返回 `REG_BADPAT`
- Case 内存不足：返回 `REG_ESPACE`

**REG_ICASE 处理**：对字面量字符，创建并行的 `toupper`/`tolower` UNION 节点。

**`\x{HHHH}` 扩展**：支持最多 8 位十六进制大括号形式（musl 扩展，非 POSIX）。

### tre_parse

```c
static reg_errcode_t tre_parse(tre_parse_ctx_t *ctx)
```

[Visibility]: Internal — 正则表达式顶层解析器

**意图**：完整解析 BRE 或 ERE 正则表达式，构建 AST 树。支持嵌套子表达式 `\(...\)` / `(...)` 和选择 `\|` / `|`。

**系统算法**（基于显式栈的迭代式解析）：

1. 栈初始化：第一层子匹配 ID=0 入栈
2. 主循环：
   - 遇到 `\(`(BRE) 或 `(`(ERE)：新子表达式开始 → 入栈当前 `nunion`、`nbranch`、新 `subid`
   - 断言/空标记字符串结束 → 生成空字面量
   - 否则：调用 `parse_atom` 解析原子
   - 检查后续重复运算符 `*`/`+`/`?`/`{...}` → 包装为 ITERATION 节点
   - 将原子/迭代节点拼接到当前 Branch
   - 遇到 `|` 或 `\)`/`)` 或 字符串结束 → 将当前 Branch 加入并集
   - 子表达式结束时 → 调用 `marksub` 标记 → 出栈恢复到上层

**语法支持**（扩展）：
- BRE 中 `\|` 作为选择、`\+`/`\?` 作为重复（musl 扩展）
- 空分支（如 `()`、`(a|)`）被接受但匹配空串
- 连续的重复（如 `a++`）被接受

**前置条件**：
- `ctx` 不为 NULL，`ctx->start` 指向正则表达式字符串
- `ctx->stack` 为有效栈
- `ctx->mem` 为有效分配器

**后置条件**：
- Case 成功 (`REG_OK`)：`ctx->n` 指向完整的 AST 根节点，`ctx->submatch_id` 为子匹配总数
- Case 括号不匹配：返回 `REG_EPAREN`
- Case 非法 `\{\}` 语法：返回 `REG_BADBR`
- Case 其他解析错误：传播 `parse_atom` 的错误码
- Case 内存不足：返回 `REG_ESPACE`

---

## 十、内部辅助函数（Tag 注入 - 子匹配位置标记）

### tre_add_tag_left

```c
static reg_errcode_t
tre_add_tag_left(tre_mem_t mem, tre_ast_node_t *node, int tag_id)
```

[Visibility]: Internal — 在节点左侧插入标签

**意图**：创建一个新的 TAG 字面量作为左子节点，原节点内容作为右子节点，插入到 CATENATION 结构。原节点类型被替换为 CATENATION。

**前置条件**：
- `mem` 不为 NULL
- `node` 不为 NULL

**后置条件**：
- Case 成功：`node->type == CATENATION`，左子为 TAG(node, tag_id)，右子为原节点内容
- Case 失败（内存不足）：返回 `REG_ESPACE`

### tre_add_tag_right

```c
static reg_errcode_t
tre_add_tag_right(tre_mem_t mem, tre_ast_node_t *node, int tag_id)
```

[Visibility]: Internal — 在节点右侧插入标签

同 `tre_add_tag_left`，但标签插入在右侧。主要用于 UNION 的非贪婪标记。

### tre_purge_regset

```c
static void
tre_purge_regset(int *regset, tre_tnfa_t *tnfa, int tag)
```

[Visibility]: Internal — 清除并应用寄存器集

**意图**：遍历 `regset`，将其中记录的每个子匹配的 `so_tag` 或 `eo_tag` 设置为 `tag`，然后清空 `regset`（设 `regset[0] = -1`）。

**前置条件**：
- `regset` 以 -1 终止
- `tnfa` 不为 NULL（`submatch_data` 已分配）

**不变量**：`regset[i]` 为偶数表示起始标签 (`i*2`)，为奇数表示结束标签 (`i*2+1`)

### tre_add_tags

```c
static reg_errcode_t
tre_add_tags(tre_mem_t mem, tre_stack_t *stack, tre_ast_node_t *tree, tre_tnfa_t *tnfa)
```

[Visibility]: Internal — 子匹配标签注入算法

**意图**：两遍遍历 AST 树，为子匹配表达式插入标签节点（TAG literal）。

**系统算法**（基于显式栈的后序遍历，分两遍）：

**第一遍** (`mem == NULL || tnfa == NULL`)：
- 计算每个 AST 节点的 `num_tags`（需要的标签数）
- 不实际修改 AST 结构

**第二遍**：
- 为每个需要标记的位置插入 TAG 字面量
- 处理 LITERAL/BACKREF 前的标签插入
- 处理 ITERATION 的标签（最小匹配标签特殊处理）
- 处理 UNION 的标签（左右子树隔离，事后标签最大化）
- 处理 CATENATION 的标签传递
- 跟踪 `minimal_tags` 映射表

**前置条件**：
- `stack` 不为 NULL
- `tree` 不为 NULL
- `regset`、`parents`、`saved_states` 在第二遍时已分配

**后置条件**：
- Case 成功：`tnfa->num_tags`、`tnfa->num_minimals`、`tnfa->end_tag` 设置完毕，`tnfa->tag_directions`、`tnfa->minimal_tags`、`tnfa->submatch_data[*].so_tag/eo_tag`、`tnfa->submatch_data[*].parents` 填充完毕
- `tree->num_tags == num_tags`（断言保证）
- Case 失败（内存不足）：返回 `REG_ESPACE`

---

## 十一、内部辅助函数（AST 变换）

### tre_copy_ast

```c
static reg_errcode_t
tre_copy_ast(tre_mem_t mem, tre_stack_t *stack, tre_ast_node_t *ast,
             int flags, int *pos_add, tre_tag_direction_t *tag_directions,
             tre_ast_node_t **copy, int *max_pos)
```

[Visibility]: Internal — AST 子树深拷贝

**意图**：递归（基于显式栈）复制 AST 子树，支持标签移除和首个标签最大化模式。

**flag 参数**：
- `COPY_REMOVE_TAGS`：将 TAG 节点替换为 EMPTY
- `COPY_MAXIMIZE_FIRST_TAG`：首个遇到的 TAG 节点标记为最大化方向

**前置条件**：
- `mem`、`stack`、`ast`、`copy`、`pos_add`、`max_pos` 不为 NULL

**后置条件**：
- `*copy` 指向深拷贝的 AST 根节点
- `*pos_add` 增加拷贝的字面量/反向引用节点数
- `*max_pos` 更新为遇到的最大 position 值

### tre_expand_ast

```c
static reg_errcode_t
tre_expand_ast(tre_mem_t mem, tre_stack_t *stack, tre_ast_node_t *ast,
               int *position, tre_tag_direction_t *tag_directions)
```

[Visibility]: Internal — 迭代节点展开

**意图**：将 `{m,n}` 迭代节点展开为可能的匹配序列。例如 `a{3}` 展开为 `aaa`，`a{2,4}` 展开为 `aa(a(a|)|)`。

**系统算法**：
- 对每个 ITERATION 节点：
  - 若 `min > 1` 或 `max > 1`：
    1. 创建 `min` 个副本的连接（除最后副本外移除标签）
    2. 若 `max == -1`：创建 `arg*`（`arg` 的星号迭代）
    3. 若 `max` 有限：创建 `(empty | copy)` 的级联
  - 将展开结果合并回原节点位置
- 更新所有 LITERAL 的 `position` 域

**前置条件**：
- `mem`、`stack`、`ast`、`position` 不为 NULL

**后置条件**：
- `*position` 递增已展开的位置总数
- AST 树中无 `min > 1` 或 `max > 1` 的 ITERATION 节点（展开后 `max` 要么为 -1 要么为 1）

---

## 十二、内部辅助函数（位置集合操作）

### tre_set_empty

```c
static tre_pos_and_tags_t *tre_set_empty(tre_mem_t mem)
```

[Visibility]: Internal — 创建空位置集合

**意图**：创建仅含终止元素 `{-1, -1, -1}` 的位置集合。

### tre_set_one

```c
static tre_pos_and_tags_t *
tre_set_one(tre_mem_t mem, int position, int code_min, int code_max,
            tre_ctype_t class, tre_ctype_t *neg_classes, int backref)
```

[Visibility]: Internal — 创建单元素位置集合

**意图**：创建包含一个位置及终止元素的位置集合。

### tre_set_union

```c
static tre_pos_and_tags_t *
tre_set_union(tre_mem_t mem, tre_pos_and_tags_t *set1, tre_pos_and_tags_t *set2,
              int *tags, int assertions)
```

[Visibility]: Internal — 两个位置集合的并集

**意图**：合并 `set1` 和 `set2`。对于 `set1` 的元素，附加 `tags` 和 `assertions`；`set2` 的元素保持原有 tags。断言位通过按位或合并。

**前置条件**：
- `set1`、`set2` 均以 `position = -1` 终止
- `tags` 以 -1 终止（可为 NULL）

---

## 十三、内部辅助函数（NFL 计算 - Nullable/Firstpos/Lastpos）

### tre_match_empty

```c
static reg_errcode_t
tre_match_empty(tre_stack_t *stack, tre_ast_node_t *node, int *tags,
                int *assertions, int *num_tags_seen)
```

[Visibility]: Internal — 计算通过 AST 可空路径的标签和断言

**意图**：遍历 AST 寻找可匹配空串的路径，收集路径上的 TAG 节点和 ASSERTION 节点。

**系统算法**（悲观优先规则）：
- LITERAL：TAG → 收集；ASSERTION → 合并断言位；EMPTY → 通过
- UNION：优先左侧可空路径（POSIX 左优先规则）
- CATENATION：必须通过两侧
- ITERATION：若参数可空，通过参数

**前置条件**：
- `stack` 不为 NULL
- `node` 不为 NULL

**后置条件**：
- `tags`（如果不为 NULL）：填充路径上的标签列表（去重），以 -1 终止
- `assertions`（如果不为 NULL）：合并路径上的所有断言位
- `num_tags_seen`（如果不为 NULL）：设为看到的标签数

### tre_compute_nfl

```c
static reg_errcode_t
tre_compute_nfl(tre_mem_t mem, tre_stack_t *stack, tre_ast_node_t *tree)
```

[Visibility]: Internal — Nullable/Firstpos/Lastpos 计算

**意图**：对 AST 每个节点计算 `nullable`、`firstpos`、`lastpos` 属性，为后续 TNFA 构建做准备。

**语义定义**：
- **nullable**：该子树能否匹配空串（1=可，0=不可）
- **firstpos**：该子树匹配的第一个字符可能来自的位置集合
- **lastpos**：该子树匹配的最后一个字符可能来自的位置集合

**系统算法**（自底向上）：

| 节点类型 | nullable | firstpos | lastpos |
|---------|----------|----------|---------|
| 字面量(pos=i) | 0 | {i} | {i} |
| BACKREF(pos=i) | 0 | {i} | {i} + backref 断言 |
| TAG/ASSERTION/EMPTY | 1 | {} | {} |
| UNION | left \|\| right | left U right | left U right |
| ITERATION | min==0 \|\| arg.nullable | arg.firstpos | arg.lastpos |
| CATENATION | left && right | left.nullable ? right U left_tags(left) : left | right.nullable ? left U right_tags(right) : right |

**前置条件**：
- `mem`、`stack`、`tree` 不为 NULL

**后置条件**：
- 所有节点的 `nullable`、`firstpos`、`lastpos` 计算完毕
- Case 失败（内存不足）：返回 `REG_ESPACE`

---

## 十四、内部辅助函数（TNFA 构建）

### tre_make_trans

```c
static reg_errcode_t
tre_make_trans(tre_pos_and_tags_t *p1, tre_pos_and_tags_t *p2,
               tre_tnfa_transition_t *transitions, int *counts, int *offs)
```

[Visibility]: Internal — 创建位置间的 TNFA 转移

**意图**：从 `p1` 每个位置向 `p2` 每个位置创建转移边。

**两遍设计**：
- 若 `transitions == NULL`：仅统计每个位置出边的数量（存入 `counts`）
- 若 `transitions != NULL`：实际填充转移结构

**前置条件**：
- `p1`、`p2` 以 `position = -1` 终止

**后置条件**：
- 每个 `(p1[i], p2[j])` 组合创建一条转移，包含：
  - `code_min`/`code_max`：字符范围
  - `state`/`state_id`：目标状态
  - `assertions`：合并的断言位（含 CHAR_CLASS 标记）
  - `tags`：合并的去重标签列表
  - `u.backref`/`u.class`：反向引用/字符类
  - `neg_classes`：否定字符类列表

### tre_ast_to_tnfa

```c
static reg_errcode_t
tre_ast_to_tnfa(tre_ast_node_t *node, tre_tnfa_transition_t *transitions,
                int *counts, int *offs)
```

[Visibility]: Internal — AST 到 TNFA 的编译

**意图**：递归遍历 AST，为每个 CATENATION 和 ITERATION 节点创建转移边。

**系统算法**（递归）：
- LITERAL：无操作（转移由父亲创建）
- UNION：递归处理左右子
- CATENATION：连接 `left.lastpos` 到 `right.firstpos`（调用 `tre_make_trans`），然后递归处理左右
- ITERATION（max == -1）：连接 `arg.lastpos` 到 `arg.firstpos`（形成循环），然后递归处理 `arg`

**注意**：当前实现使用函数递归而非显式栈，注释中标明此为已知 TODO。

**前置条件**：
- `node` 的 `firstpos`/`lastpos` 已计算（`tre_compute_nfl` 必须已执行）

**后置条件**：
- 两遍：第一遍统计出边数量，第二遍填充转移表

---

## 十五、对外导出函数

### regcomp

```c
int regcomp(regex_t *restrict preg, const char *restrict regex, int cflags)
```

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。用户程序可直接调用。

**意图**：将正则表达式字符串 `regex` 编译为内部格式，存入 `preg`。编译结果供 `regexec` 使用。

**编译管线**（Level 1 复杂度，全流程描述）：

```
输入: regex 字符串 + cflags
  1. 分配栈 (tre_stack_new) 与内存池 (tre_mem_new)
  2. 解析 (tre_parse) → AST 树
  3. 校验反向引用不越界 (max_backref <= re_nsub)
  4. 标签注入 (tre_add_tags, 两遍) → 子匹配位置标记
  5. 迭代展开 (tre_expand_ast) → {m,n} → 连接/并集
  6. 附加最终状态哑节点
  7. NFL 计算 (tre_compute_nfl) → nullable/firstpos/lastpos
  8. TNFA 转移统计 (tre_ast_to_tnfa, 第一遍)
  9. 转移表分配与偏移计算
  10. TNFA 转移填充 (tre_ast_to_tnfa, 第二遍)
  11. 初始状态转移表构建 (从 tree->firstpos)
  12. 清理临时资源，TNFA 存入 preg->__opaque
输出: preg（已编译正则表达式）
```

**前置条件**：
- `preg != NULL`：`regex_t` 指针有效且未保存编译结果（或已被 `regfree` 释放）
- `regex != NULL`：指向以 `\0` 结尾的正则表达式字符串
- `cflags` 由以下标志按位或构成：
  - `REG_EXTENDED`：使用 ERE 语法，否则使用 BRE
  - `REG_ICASE`：大小写不敏感匹配
  - `REG_NOSUB`：不需要子匹配信息（可跳过标签注入优化）
  - `REG_NEWLINE`：特殊对待换行符（`.` 不匹配 `\n`，`[^...]` 不排除 `\n`）

**后置条件**：
- Case 成功（返回 `REG_OK` = 0）：
  - `preg->re_nsub` 设为子表达式数量
  - `preg->TRE_REGEX_T_FIELD` (`__opaque`) 指向编译好的 TNFA 结构
  - 所有临时内存已释放
- Case 失败（返回错误码 != 0）：
  - `preg->TRE_REGEX_T_FIELD` 可能被置为非 NULL（部分构造的 TNFA）
  - 调用 `regfree(preg)` 清理残留资源
  - `preg` 状态未定义，不建议用于匹配

**错误码对应**：
| 错误码 | 含义 |
|--------|------|
| `REG_OK` (0) | 成功 |
| `REG_NOMATCH` (1) | （编译阶段不使用） |
| `REG_BADPAT` (2) | 正则表达式语法错误（非法字节序列等） |
| `REG_ECOLLATE` (3) | 无效排序元素（`[.`、`[=` 不支持） |
| `REG_ECTYPE` (4) | 无效字符类名 |
| `REG_EESCAPE` (5) | 尾部转义符（`\` 位于末尾） |
| `REG_ESUBREG` (6) | 引用不存在的子表达式（`\2` 但只有 1 组） |
| `REG_EBRACK` (7) | 括号不匹配（缺 `]`） |
| `REG_EPAREN` (8) | 括号不匹配（缺 `)` 或 `\)`） |
| `REG_EBRACE` (9) | 花括号不匹配（`\x{HHHH` 缺 `}`） |
| `REG_BADBR` (10) | 非法 `\{m,n\}` 语法 |
| `REG_ERANGE` (11) | 非法字符范围（如 `z-a`） |
| `REG_ESPACE` (12) | 内存不足 |
| `REG_BADRPT` (13) | 非法重复运算符（如 `*` 出现在表达式开头） |

**不变量**：
- 编译过程不修改 `regex` 字符串内容
- 若编译成功返回，`preg` 可用于 `regexec`；若失败，调用者应调用 `regfree(preg)` 释放

**依赖**：
- `tre_mem_new` / `tre_mem_destroy`（see tre-mem.c spec）：快速分配器
- `tre_stack_new` / `tre_stack_destroy`（本文件内部）：动态栈
- `tre_parse`（本文件内部）：解析器
- `tre_add_tags`（本文件内部）：标签注入
- `tre_expand_ast`（本文件内部）：迭代展开
- `tre_compute_nfl`（本文件内部）：NFL 计算
- `tre_ast_to_tnfa`（本文件内部）：TNFA 编译
- `xmalloc`/`xcalloc`/`xfree`/`xrealloc`（→ `stdlib.h` malloc/calloc/free/realloc）
- `regfree`（本文件，回滚时调用）

---

### regfree

```c
void regfree(regex_t *preg)
```

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。用户程序可直接调用。

**意图**：释放 `regcomp` 编译产生的所有内存资源。调用后 `preg` 可被重新用于下一次 `regcomp` 或丢弃。

**前置条件**：
- `preg != NULL`
- `preg->TRE_REGEX_T_FIELD` 要么为 NULL（未编译或已释放），要么指向有效的 `tre_tnfa_t` 结构

**后置条件**：
- 若 `preg->TRE_REGEX_T_FIELD == NULL`：无操作（空操作安全）
- 否则：
  - 遍历释放所有转移的 `tags` 和 `neg_classes`
  - 释放 `tnfa->transitions` 数组
  - 遍历释放初始转移的 `tags`，释放 `tnfa->initial`
  - 遍历释放每个子匹配数据的 `parents` 数组
  - 释放 `tnfa->submatch_data`、`tnfa->tag_directions`、`tnfa->firstpos_chars`、`tnfa->minimal_tags`
  - 释放 `tnfa` 结构体本身
- `preg` 不再持有任何动态分配的引用（TNFA 已销毁）

**不变量**：
- 多次调用 `regfree(preg)` 是安全的（第二次为 NULL 时无操作）
- `regfree` 不抛出错误，不返回错误码

**依赖**：
- `xfree`（→ `stdlib.h` free）
- `tre_tnfa_t` 结构定义（see tre.h spec）

---

## 十六、宏定义

### 特殊节点判断宏

```c
#define IS_SPECIAL(x)   ((x)->code_min < 0)
#define IS_EMPTY(x)     ((x)->code_min == EMPTY)
#define IS_ASSERTION(x) ((x)->code_min == ASSERTION)
#define IS_TAG(x)       ((x)->code_min == TAG)
#define IS_BACKREF(x)   ((x)->code_min == BACKREF)
```

[Visibility]: Internal — 通过 `code_min` 负值编码判断特殊节点子类型

### ERROR_EXIT 宏

```c
#define ERROR_EXIT(err)  do { errcode = err; goto error_exit; } while(0)
```

[Visibility]: Internal — regcomp 函数内错误退出跳转，设置 errcode 并跳转到统一清理标签

### STACK 宏系列

```c
#define STACK_PUSH(s, typetag, value)   // 推入但不检查返回值
#define STACK_PUSHX(s, typetag, value)  // 推入，失败则 break
#define STACK_PUSHR(s, typetag, value)  // 推入，失败则 return
```

[Visibility]: Internal — 栈操作的便捷宏，分别对应"忽略错误"/"break 退出循环"/"return 返回错误码"三种错误处理策略

---

## 跨文件依赖声明

| 依赖 | 来源 | 可见性 |
|------|------|--------|
| `tre_mem_new_impl` / `__tre_mem_new_impl` | `tre-mem.c` | Internal (hidden) |
| `tre_mem_alloc_impl` / `__tre_mem_alloc_impl` | `tre-mem.c` | Internal (hidden) |
| `tre_mem_destroy` / `__tre_mem_destroy` | `tre-mem.c` | Internal (hidden) |
| `tre_mem_t` / `struct tre_mem_struct` | `tre.h` | Internal |
| `tre_tnfa_t` / `tre_tnfa_transition_t` / `tre_submatch_data_t` | `tre.h` | Internal |
| `tre_tag_direction_t` / `tre_cint_t` / `tre_ctype_t` | `tre.h` | Internal |
| `ASSERT_AT_BOL` / `ASSERT_AT_EOL` 等断言宏 | `tre.h` | Internal |
| `regex_t` / `reg_errcode_t` / `REG_*` 错误码 | `<regex.h>` | Public |
| `RE_DUP_MAX` | `<regex.h>` | Public (POSIX) |
| `CHARCLASS_NAME_MAX` | `<regex.h>` | Public (POSIX) |
| `xmalloc`/`xcalloc`/`xfree`/`xrealloc` | → `stdlib.h` 的 malloc/calloc/free/realloc | Public (libc) |
| `mbtowc` / `memset` / `memcpy` / `qsort` / `isdigit` | `<stdlib.h>` / `<string.h>` / `<ctype.h>` | Public (libc) |
| `tre_isalnum` → `iswalnum` 等宽字符分类宏 | `<wctype.h>`（通过 `tre.h` 间接引用） | Public (libc) |
| `tre_ctype` → `wctype` | `<wctype.h>`（通过 `tre.h` 间接引用） | Public (libc) |

---

## 安全与正确性属性

1. **两遍标签注入**：`tre_add_tags` 第一遍仅计数（`mem == NULL`），第二遍实际修改 AST。两遍之间分配的资源量保证了栈操作不会因 realloc 导致性能退化。
2. **AST 展开的保守性**：`tre_expand_ast` 对 `{m,n}` 可能产生指数级节点数，但受 `RE_DUP_MAX` 约束。
3. **显式栈替代递归**：部分函数（`tre_parse`、`tre_add_tags`、`tre_copy_ast`、`tre_expand_ast`、`tre_compute_nfl`、`tre_match_empty`）使用显式栈进行后序遍历，避免深度嵌套正则表达式导致的栈溢出。但 `tre_ast_to_tnfa` 仍使用函数递归（已知 TODO）。
4. **内存安全**：所有分配路径在失败时通过 `error_exit` 标签统一清理，调用 `regfree` 释放已部分构造的 TNFA。
5. **POSIX 左优先语义**：UNION 节点的处理保证左侧分支优先于右侧（在 `tre_match_empty` 的空路径选择、`tre_set_union` 的标签附加、`tre_add_tags` 的标签分配中均有体现）。
