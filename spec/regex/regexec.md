# regexec.c 规约

## 依赖图

```
regexec (Public)
  ├── tre_fill_pmatch (Internal) ─── tre_submatch_data_t (tre.h)
  ├── tre_tnfa_run_backtrack (Internal)
  │     ├── tre_tag_order (Internal)
  │     ├── tre_fill_pmatch (Internal)
  │     ├── GET_NEXT_WCHAR (macro)
  │     ├── CHECK_ASSERTIONS (macro)
  │     ├── CHECK_CHAR_CLASSES (macro) ─── tre_neg_char_classes_match (Internal)
  │     ├── BT_STACK_PUSH (macro)
  │     ├── BT_STACK_POP (macro)
  │     ├── tre_bt_mem_new ──> tre_mem_new_impl (see tre-mem.c)
  │     ├── tre_bt_mem_alloc ──> tre_mem_alloc_impl (see tre-mem.c)
  │     └── tre_bt_mem_destroy ──> tre_mem_destroy (see tre-mem.c)
  └── tre_tnfa_run_parallel (Internal)
        ├── tre_tag_order (Internal)
        ├── GET_NEXT_WCHAR (macro)
        ├── CHECK_ASSERTIONS (macro)
        └── CHECK_CHAR_CLASSES (macro) ─── tre_neg_char_classes_match (Internal)

类型依赖:
  regexec ──> regex_t, regmatch_t (see <regex.h>)
  regexec ──> tre_tnfa_t, tre_submatch_data_t (see tre.h)
  tre_tnfa_run_parallel ──> tre_tnfa_reach_t, tre_reach_pos_t (regexec.c 内部)
  tre_tnfa_run_backtrack ──> tre_backtrack_item_t, tre_backtrack_t (regexec.c 内部)

外部模块依赖:
  tre_mem_new_impl, tre_mem_alloc_impl, tre_mem_destroy (see tre-mem.c spec)
  calloc, free, memset, strncmp (libc)
  mbtowc (libc, wchar.h)
  iswalnum, towlower, towupper, iswctype (libc, wctype.h)
```

---

## 内部类型定义

### tre_tnfa_reach_t (内部类型)

[Visibility]: Internal — musl TRE 内部并行匹配器使用的可达状态结构，POSIX/C 标准未定义

```c
typedef struct {
  tre_tnfa_transition_t *state;
  regoff_t *tags;
} tre_tnfa_reach_t;
```

- **设计意图**: 表示并行匹配器 (`tre_tnfa_run_parallel`) 中某一个"可达路径"的状态。每条路径由当前所处的 TNFA 状态指针 `state` 和一个标签值数组 `tags`（记录各捕获组的起始偏移）组成。
- 仅作为数组元素在 `tre_tnfa_run_parallel` 内部使用，两个数组 `reach` 和 `reach_next` 交替表示当前和下一轮的可达状态集合。

---

### tre_reach_pos_t (内部类型)

[Visibility]: Internal — musl TRE 内部并行匹配器使用的"每状态已访问记录"结构，POSIX/C 标准未定义

```c
typedef struct {
  regoff_t pos;
  regoff_t **tags;
} tre_reach_pos_t;
```

- **设计意图**: 记录 TNFA 某个 state_id 最近一次被访问时的字符位置 `pos` 和最佳标签数组指针 `tags`。用于"当多条路径到达同一状态时，按标签顺序规则择一保留"的去重优化。
- 仅在 `tre_tnfa_run_parallel` 内部使用，按 state_id 索引。

---

### tre_backtrack_item_t (内部类型)

[Visibility]: Internal — musl TRE 内部回溯匹配器使用的栈帧结构，POSIX/C 标准未定义

```c
typedef struct {
  regoff_t pos;
  const char *str_byte;
  tre_tnfa_transition_t *state;
  int state_id;
  int next_c;
  regoff_t *tags;
  /* mbstate_t mbstate; 仅在 TRE_MBSTATE 定义时存在，musl 中 #undef TRE_MBSTATE */
} tre_backtrack_item_t;
```

- **设计意图**: 回溯匹配器栈中的一个帧，保存回溯点的完整上下文：字符位置、字符串指针、当前 TNFA 状态、下一个宽字符预览、标签数组。当当前探索路径失败时，从栈顶恢复这些上下文继续尝试其他分支。

---

### tre_backtrack_struct / tre_backtrack_t (内部类型)

[Visibility]: Internal — musl TRE 内部回溯匹配器的双向链表栈结构，POSIX/C 标准未定义

```c
typedef struct tre_backtrack_struct {
  tre_backtrack_item_t item;
  struct tre_backtrack_struct *prev;
  struct tre_backtrack_struct *next;
} *tre_backtrack_t;
```

- **设计意图**: 回溯栈的实现。使用双向链表而非数组，`prev` 指向下层（栈顶方向），`next` 指向已分配的备用节点。其设计巧妙之处在于：`BT_STACK_PUSH` 若 `stack->next` 非空则复用已分配内存，避免频繁 malloc/free。`tre_mem_alloc` 管理所有分配的节点内存，栈销毁时一次性释放。

---

## 内部宏定义

### GET_NEXT_WCHAR() (内部宏)

[Visibility]: Internal — musl TRE 内部字符读取宏，POSIX/C 标准未定义

```c
#define GET_NEXT_WCHAR() do {
    prev_c = next_c;
    pos += pos_add_next;
    if ((pos_add_next = mbtowc(&next_c, str_byte, MB_LEN_MAX)) <= 0) {
        if (pos_add_next < 0) { ret = REG_NOMATCH; goto error_exit; }
        else pos_add_next++;
    }
    str_byte += pos_add_next;
} while (0)
```

- **前置条件**:
  - `str_byte` 指向当前输入字符串位置
  - `pos` 为当前的字符偏移量
  - 调用此宏的函数的局部作用域内存在 `ret`、`error_exit` 标签
- **后置条件 (Case 1: 成功)**:
  - `prev_c` = 上一个字符（宽字符表示），`next_c` = 新读取的宽字符
  - `pos` 前进一位（以字符计数），`str_byte` 前进该字符的字节数
- **后置条件 (Case 2: 解码错误)**:
  - 若 `mbtowc()` 返回负值 -> `ret = REG_NOMATCH`，跳转到 `error_exit`
- **后置条件 (Case 3: NUL 终止符)**:
  - 若 `next_c = L'\0'`，表示字符串结束，调用方需自行检查

---

### IS_WORD_CHAR(c) (内部宏)

[Visibility]: Internal — musl TRE 内部辅助宏，POSIX/C 标准未定义

```c
#define IS_WORD_CHAR(c) ((c) == L'_' || tre_isalnum(c))
```

其中 `tre_isalnum` 宏展开为 `iswalnum`（libc 宽字符函数）。

- **前/后置条件**: 纯函数式，无副作用。若 `c` 是下划线 `_` 或是字母数字宽字符，返回非零（真）；否则返回零（假）。

---

### CHECK_ASSERTIONS(assertions) (内部宏)

[Visibility]: Internal — musl TRE 内部断言检查宏，POSIX/C 标准未定义

```c
#define CHECK_ASSERTIONS(assertions)
  (((assertions & ASSERT_AT_BOL)
    && (pos > 0 || reg_notbol)
    && (prev_c != L'\n' || !reg_newline))
   || ((assertions & ASSERT_AT_EOL)
       && (next_c != L'\0' || reg_noteol)
       && (next_c != L'\n' || !reg_newline))
   || ((assertions & ASSERT_AT_BOW)
       && (IS_WORD_CHAR(prev_c) || !IS_WORD_CHAR(next_c)))
   || ((assertions & ASSERT_AT_EOW)
       && (!IS_WORD_CHAR(prev_c) || IS_WORD_CHAR(next_c)))
   || ((assertions & ASSERT_AT_WB)
       && (pos != 0 && next_c != L'\0'
           && IS_WORD_CHAR(prev_c) == IS_WORD_CHAR(next_c)))
   || ((assertions & ASSERT_AT_WB_NEG)
       && (pos == 0 || next_c == L'\0'
           || IS_WORD_CHAR(prev_c) != IS_WORD_CHAR(next_c))))
```

- **前置条件**:
  - `pos` 为当前字符位置（0 表示字符串起始）
  - `prev_c` 和 `next_c` 为当前和前一个宽字符
  - `reg_notbol`、`reg_noteol`、`reg_newline` 为匹配标志
  - `assertions` 为整数位掩码，由 TNFA 转换的 `assertions` 字段携带
- **后置条件**: 返回值含义
  - 返回非零（真）：**断言检查失败**，即该转换在当前上下文下不应被激活（语义上表示"拒绝"）
  - 返回零（假）：断言检查通过，该转换为有效
- **断言含义**:
  - `ASSERT_AT_BOL` (1): 要求当前位置是行首。`pos==0` 且 `!reg_notbol`，或前一个字符是换行符且 `reg_newline`
  - `ASSERT_AT_EOL` (2): 要求当前位置是行尾。`next_c=='\0'` 且 `!reg_noteol`，或下一个字符是换行符且 `reg_newline`
  - `ASSERT_AT_BOW` (16): 要求当前位置是单词边界（词首侧）。前一个是非单词字符且当前是单词字符
  - `ASSERT_AT_EOW` (32): 要求当前位置是单词边界（词尾侧）。前一个是单词字符且当前是非单词字符
  - `ASSERT_AT_WB` (64): 要求当前位置是单词内部（非边界）。前后字符的"单词性"一致
  - `ASSERT_AT_WB_NEG` (128): 要求当前位置不是单词边界。`pos==0` 或 `next_c=='\0'` 或前后字符"单词性"不同

---

### CHECK_CHAR_CLASSES(trans_i, tnfa, eflags) (内部宏)

[Visibility]: Internal — musl TRE 内部字符类断言检查宏，POSIX/C 标准未定义

```c
#define CHECK_CHAR_CLASSES(trans_i, tnfa, eflags)
  (((trans_i->assertions & ASSERT_CHAR_CLASS)
       && !(tnfa->cflags & REG_ICASE)
       && !tre_isctype((tre_cint_t)prev_c, trans_i->u.class))
    || ((trans_i->assertions & ASSERT_CHAR_CLASS)
        && (tnfa->cflags & REG_ICASE)
        && !tre_isctype(tre_tolower((tre_cint_t)prev_c), trans_i->u.class)
        && !tre_isctype(tre_toupper((tre_cint_t)prev_c), trans_i->u.class))
    || ((trans_i->assertions & ASSERT_CHAR_CLASS_NEG)
        && tre_neg_char_classes_match(trans_i->neg_classes, (tre_cint_t)prev_c,
                                      tnfa->cflags & REG_ICASE)))
```

- **前置条件**: `prev_c` 为当前输入字符，`trans_i` 指向当前 TNFA 转换，`tnfa` 为编译后的 NFA
- **后置条件**: 返回值含义
  - 返回非零（真）：**字符类检查失败**（当前字符不属于要求的类）
  - 返回零（假）：字符类检查通过
- **语义分拆**:
  1. `ASSERT_CHAR_CLASS` (4): 正向字符类 \([ ... ]\)。大小写不敏感时分别用小写和大写形式检查
  2. `ASSERT_CHAR_CLASS_NEG` (8): 否定字符类 \([^ ... ]\)。调用 `tre_neg_char_classes_match()` 检查

---

### BT_STACK_PUSH(...) / BT_STACK_POP() (内部宏)

[Visibility]: Internal — musl TRE 内部回溯栈操作宏，POSIX/C 标准未定义

```c
#define BT_STACK_PUSH(_pos, _str_byte, _str_wide, _state, _state_id, _next_c, _tags, _mbstate)
#define BT_STACK_POP()
```

- **意图**: 实现回溯匹配器的状态保存/恢复机制。`BT_STACK_PUSH` 将当前上下文推入栈；若 `stack->next` 已有分配则复用，否则通过 `tre_bt_mem_alloc` 分配新节点（使用 TRE 内存分配器的批量管理）。
- **BT_STACK_PUSH 前置条件**:
  - `stack` 指向当前栈顶
  - `mem` 指向有效的 TRE 内存分配器（`tre_mem_t`）
  - `tnfa` 指向编译后的 TNFA（需要 `num_tags` 确定标签数组大小）
- **BT_STACK_PUSH 后置条件**:
  - 新的栈帧 `stack` 保存了所有传入参数，`prev` 指向原栈顶
  - 若分配失败，释放所有已分配资源后返回 `REG_ESPACE`
- **BT_STACK_POP 前置条件**:
  - `stack->prev` 非 NULL（存在下层帧）
  - `pos`, `str_byte`, `state`, `next_c`, `tags` 等局部变量在作用域内
- **BT_STACK_POP 后置条件**:
  - 所有局部变量恢复为帧中保存的值
  - `stack` 回退到 `prev`

---

### MIN(a, b) (内部宏，局部重定义)

[Visibility]: Internal — `tre.h` 中已定义 `#define MIN(a,b)`，此处 `#undef MIN` 后重定义。仅在 `tre_tnfa_run_backtrack` 词法作用域内有效。

```c
#define MIN(a, b) ((a) <= (b) ? (a) : (b))
```

注意：与 `tre.h` 中的定义不同，tre.h 的版本为 `(((a) <= (b)) ? (a) : (b))`（外层多了一层括号），语义等价但此处的两重括号更"保守"。

---

## 内部辅助函数（底层依赖）

### tre_neg_char_classes_match (内部函数)

[Visibility]: Internal — musl TRE 内部辅助函数，POSIX/C 标准未定义

```c
static int
tre_neg_char_classes_match(tre_ctype_t *classes, tre_cint_t wc, int icase);
```

- **意图**: 检查给定宽字符 `wc` 是否属于否定字符类列表 `classes` 中的任一字符类。`classes` 是以 0 结尾的 `tre_ctype_t` 数组。
- **复杂度**: Level 1（前置/后置条件）
- **前置条件**:
  - `classes` 指向以 0 结尾的 `tre_ctype_t` 数组，或可为 NULL（但调用方仅在 `ASSERT_CHAR_CLASS_NEG` 激活时传入）
  - `wc` 为待检查的宽字符
  - `icase` 为非零表示大小写不敏感匹配模式
- **后置条件 (Case 1: 匹配成功)**:
  - 返回值 = 1：`wc` 属于 `classes` 中至少一个字符类
  - 大小写不敏感时：`wc` 的大写或小写形式之一属于某字符类即算匹配
- **后置条件 (Case 2: 无匹配)**:
  - 返回值 = 0：`wc` 不属于 `classes` 中的任何一个字符类

---

### tre_tag_order (内部函数)

[Visibility]: Internal — musl TRE 内部标签排序比较函数，POSIX/C 标准未定义

```c
static int
tre_tag_order(int num_tags, tre_tag_direction_t *tag_directions,
              regoff_t *t1, regoff_t *t2);
```

- **意图**: 比较两套标签值 `t1` 和 `t2`，按 TNFA 定义的 `tag_directions`（每个标签是 "最小化" 还是 "最大化"）逐位词典序判断 `t1` 是否 "优于" `t2`。
- **复杂度**: Level 1（前置/后置条件）
- **前置条件**:
  - `num_tags` > 0，指明标签数组的长度
  - `tag_directions` 为长度至少 `num_tags` 的数组
  - `t1` 和 `t2` 为长度至少 `num_tags` 的 `regoff_t` 数组
  - 每个 `tag_directions[i]` 取值为 `TRE_TAG_MINIMIZE` (0) 或 `TRE_TAG_MAXIMIZE` (1)
- **后置条件 (Case 1: t1 胜出)**:
  - 返回值 = 1：逐标签比较，在第一个 `t1[i] != t2[i]` 的位置，若该标签方向为 MINIMIZE 且 `t1[i] < t2[i]`，或方向为 MAXIMIZE 且 `t1[i] > t2[i]`
- **后置条件 (Case 2: t1 不胜出)**:
  - 返回值 = 0：`t2` 在所有分歧位上均不劣于 `t1`（包括两套标签完全相同的情况）
- **实现注释**: 注释掉的 `assert(0)` 暗示设计者认为调用方不太可能传入完全相同的两套标签，但实现上返回 0 保持安全。

---

### tre_fill_pmatch (内部函数)

[Visibility]: Internal — musl TRE 内部函数，将 TNFA 标签值转换为 POSIX `regmatch_t` 数组。POSIX/C 标准未定义。

```c
static void
tre_fill_pmatch(size_t nmatch, regmatch_t pmatch[], int cflags,
                const tre_tnfa_t *tnfa, regoff_t *tags, regoff_t match_eo);
```

- **意图**: 在匹配成功后，根据编译期收集的子匹配数据 (`submatch_data`) 和运行期收集的标签终点偏移 (`tags`, `match_eo`)，按左最长匹配的 POSIX 语义填充 `regmatch_t` 数组，并应用父子关系的约束修正。
- **复杂度**: Level 1（前置/后置条件 + 不变量）
- **前置条件**:
  - `nmatch` >= 0，指明 `pmatch` 数组的长度
  - `pmatch` 指向长度至少 `nmatch` 的数组，或 `nmatch == 0` 时可为 NULL
  - `tnfa` 指向编译后的 TNFA，其 `submatch_data` 和 `num_submatches` 字段有效
  - `tags` 指向长度至少 `tnfa->num_tags` 的标签数组，所有标签值为 `-1`（未使用）或非负偏移
  - `match_eo` 为匹配结束偏移，若未找到匹配则为 -1
- **后置条件**:
  - 对 `i < min(nmatch, tnfa->num_submatches)`：
    - 若 `match_eo < 0` 或 `cflags & REG_NOSUB`：`pmatch[i].rm_so = pmatch[i].rm_eo = -1`
    - 否则：`pmatch[i].rm_so` = `tags[submatch_data[i].so_tag]` 或 `match_eo`（若 `so_tag == end_tag`），`pmatch[i].rm_eo` 同理
    - 若任一端点值为 -1，则两者均设为 -1（该子匹配未参与本次匹配）
  - 对 `i >= tnfa->num_submatches` 且 `i < nmatch`：`pmatch[i].rm_so = pmatch[i].rm_eo = -1`
  - **子匹配父子约束修正（第二遍遍历）**:
    - 对每个子匹配，若有定义 `parents[j] >= 0`，检查其区间是否被父区间包含；若 `rm_so < parents_rm_so` 或 `rm_eo > parents_rm_eo`，则该子匹配重置为 `-1, -1`
  - 不变量：`pmatch[i].rm_so == -1` 蕴含 `pmatch[i].rm_eo == -1`（反之亦然）

---

## 匹配引擎

### tre_tnfa_run_parallel (内部函数)

[Visibility]: Internal — musl TRE 内部并行匹配引擎，POSIX/C 标准未定义

```c
static reg_errcode_t
tre_tnfa_run_parallel(const tre_tnfa_t *tnfa, const void *string,
                      regoff_t *match_tags, int eflags,
                      regoff_t *match_end_ofs);
```

- **意图**: 实现 POSIX 左最长匹配的并行 NFA 模拟算法。所有匹配路径同时推进，到达同一状态时按标签方向规则择一保留。该算法 **不能** 处理包含反向引用的正则表达式（此时应使用 `tre_tnfa_run_backtrack`）。
- **复杂度**: Level 3（前/后置条件 + 系统算法）
- **系统算法**:
  1. **初始化**: 分配单一大块内存 (`calloc`)，从中切分 `reach`/`reach_next` 数组（当前轮/下一轮状态可达表）、`reach_pos` 数组（每个状态最近访问的记录）和标签存储区。使用对齐填充 (`ALIGN`) 保证各指针对齐。
  2. **初始状态加入**: 扫描 `tnfa->initial` 中的初始转换，检查断言 (`CHECK_ASSERTIONS`)，通过的加入 `reach_next`，同时设置通过该转换的标签值。若直接到达 `tnfa->final` 则记录匹配。
  3. **主循环**（逐字符推进）:
     - 读取下一宽字符 (`GET_NEXT_WCHAR`)
     - 交换 `reach` 和 `reach_next` 数组
     - **最小匹配剔除**（仅当 `tnfa->num_minimals > 0` 且已有新匹配时）: 遍历 `reach`，丢弃不满足最小匹配条件的状态
     - **转换探索**: 对 `reach` 中每个状态，尝试其所有出边。若转换的字符范围匹配 `prev_c`，则：
       - 检查断言和字符类 (`CHECK_ASSERTIONS`, `CHECK_CHAR_CLASSES`)
       - 计算新标签值
       - 若目标状态未访问或本次路径更优（`tre_tag_order`），更新 `reach_next`
       - 若到达 `tnfa->final` 且本次匹配不差于已知最优，更新 `match_eo` 和 `match_tags`
  4. **终止**: 当字符串读完且 `reach_next` 为空（无更多可达状态）或 `num_tags == 0` 且已找到匹配时跳出
  5. **清理**: `xfree(buf)` 释放临时内存
- **前置条件**:
  - `tnfa` 指向已编译的 TNFA，不可为 NULL
  - `string` 指向以 NUL 结尾的多字节字符串
  - `match_tags` 可为 NULL（不需要捕获组信息）；若需捕获组，指向长度至少 `tnfa->num_tags` 的数组
  - `eflags` 为 `REG_NOTBOL | REG_NOTEOL` 的组合
  - `match_end_ofs` 指向有效的 `regoff_t` 变量，用于输出匹配结束偏移
- **后置条件 (Case 1: 匹配成功)**:
  - 返回值 = `REG_OK`
  - `*match_end_ofs` = 左最长匹配的结束字符偏移（>= 0）
  - 若 `match_tags != NULL`，`match_tags[0..num_tags-1]` 包含标签值（子匹配开始/结束偏移），未使用的标签值为 -1
- **后置条件 (Case 2: 无匹配)**:
  - 返回值 = `REG_NOMATCH`
  - `*match_end_ofs = -1`
- **后置条件 (Case 3: 内存不足)**:
  - 返回值 = `REG_ESPACE`
- **不变量**:
  - 算法保证左最长匹配：只有当 `reach_next_i` 为初始 `reach_next`（即无新可达状态）时才停止探索，确保匹配尽可能向右侧延伸
  - 标签比较保证左优先：`tre_tag_order` 确保在多个路径中选出左优先的那一条
  - 时间复杂度: O(|string| * |states| * |transitions_per_state|)，即与输入长度线性相关
- **安全性/约束**:
  - 算法验证了 `num_tags * num_states` 不会溢出，确保分配安全
  - 若输入字符串包含无效的多字节序列，`GET_NEXT_WCHAR` 返回 `REG_NOMATCH`

---

### tre_tnfa_run_backtrack (内部函数)

[Visibility]: Internal — musl TRE 内部回溯匹配引擎，POSIX/C 标准未定义

```c
static reg_errcode_t
tre_tnfa_run_backtrack(const tre_tnfa_t *tnfa, const void *string,
                       regoff_t *match_tags, int eflags,
                       regoff_t *match_end_ofs);
```

- **意图**: 实现带反向引用支持的正则表达式匹配。使用深度优先回溯搜索在 TNFA 中探索所有可能路径，确保返回左最长匹配。按 Henry Spencer 的说法，带反向引用的正则匹配是 NP 完全的，回溯是最通用的算法（尽管可能极慢甚至耗尽栈空间）。
- **复杂度**: Level 3（前/后置条件 + 系统算法）
- **系统算法**:
  1. **初始化**: 分配回溯栈 (`tre_mem` 内存分配器)、标签数组 `tags`、子匹配数组 `pmatch`（用于反向引用匹配）、`states_seen` 数组（用于检测无限的零长度反向引用循环）
  2. **try 标签**: 设置起始位置，重置标签值和 `states_seen`
  3. **初始状态处理**: 扫描 `tnfa->initial`，通过断言检查的进入探索；第一个直接作为当前 `state`，其余通过 `BT_STACK_PUSH` 保存为备选路径
  4. **主循环**:
     - **到达终态**: 比较当前匹配是否优于已知最优 (`match_eo < pos` 或 `tre_tag_order`)，若是则更新 `match_eo` 和 `match_tags`，然后无条件回溯（终态无出边）
     - **反向引用处理** (`ASSERT_BACKREF`): 调用 `tre_fill_pmatch` 获取被引用子匹配的实际区间，用 `strncmp` 与输入字符串对应位置比较。
       - 匹配成功: 跳过对应长度的输入，检查零长度反向引用的无限循环（通过 `states_seen`）
       - 匹配失败: 回溯
     - **普通字符匹配**: 读取下一字符 (`GET_NEXT_WCHAR`)，在 `state` 的出边中查找字符范围匹配 `prev_c` 的转换
       - 找到第一个匹配: 直接进入
       - 找到第二个匹配: 将当前上下文压栈 (`BT_STACK_PUSH`)，后续回溯时可转而走此分支
     - **转换失败**: 回溯 (`goto backtrack`)
  5. **回溯逻辑**: 执行 `BT_STACK_POP` 恢复上下文；若栈空且 `match_eo < 0`，将起始位置后移一位 (`goto retry`)
  6. **终止**: 当栈空且 `match_eo >= 0`（找到匹配）或字符串读完且所有起始位置均尝试完毕
- **前置条件**:
  - `tnfa` 指向已编译的 TNFA（可能包含反向引用 `have_backrefs`），不可为 NULL
  - `string` 指向以 NUL 结尾的多字节字符串
  - `match_tags` 可为 NULL
  - `eflags` 为 `REG_NOTBOL | REG_NOTEOL` 的组合
  - `match_end_ofs` 指向有效的输出变量
- **后置条件 (Case 1: 匹配成功)**:
  - 返回值 = `REG_OK`
  - `*match_end_ofs` = 左最长匹配的结束偏移
  - 若 `match_tags != NULL`，包含最优路径的标签值
- **后置条件 (Case 2: 无匹配)**:
  - 返回值 = `REG_NOMATCH`，`*match_end_ofs = -1`
- **后置条件 (Case 3: 内存不足)**:
  - 返回值 = `REG_ESPACE`
- **不变量**:
  - 匹配总是左最优: 从字符串起始位置开始线性扫描，对每个起始位置深度优先探索
  - 每个起始位置尝试结束时确保该位置的最长匹配已找到
  - 反向引用完整性: `states_seen` 防止零长度反向引用导致无限循环
- **安全性/约束**:
  - 最坏情况时间复杂度为指数级（NP 完全问题本质），但实际使用中很少触发
  - 回溯栈通过 `tre_mem` 批量管理，避免频繁 `malloc/free`
  - `BT_STACK_PUSH` 中的分配失败路径正确释放了 `tags`、`pmatch`、`states_seen`

---

## 对外导出函数

### regexec

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。用户程序可直接调用。

```c
int
regexec(const regex_t *restrict preg, const char *restrict string,
        size_t nmatch, regmatch_t pmatch[restrict], int eflags);
```

- **意图**: 对 `string` 执行 `preg` 对应的已编译正则表达式的匹配。根据正则是否包含反向引用，分派到并行匹配器（`tre_tnfa_run_parallel`）或回溯匹配器（`tre_tnfa_run_backtrack`）。
- **复杂度**: Level 2（前/后置条件 + 意图）
- **前置条件**:
  - `preg` 指向通过 `regcomp()` 成功编译的 `regex_t` 对象，且未被 `regfree()` 释放
  - `string` 指向以 NUL 结尾的多字节字符串
  - `nmatch` 为 `pmatch` 数组的元素个数（可为 0）
  - 若 `nmatch > 0`，`pmatch` 指向长度至少 `nmatch` 的有效数组
  - `eflags` 为 0 或 `REG_NOTBOL | REG_NOTEOL` 的组合
- **后置条件 (Case 1: 匹配成功)**:
  - 返回值 = `REG_OK` (= 0)
  - 若 `nmatch > 0` 且编译时未设置 `REG_NOSUB`：
    - `pmatch[0]` 包含整个匹配区间的 `rm_so`（起始偏移）和 `rm_eo`（结束偏移，即匹配字符之后的位置）
    - `pmatch[1..nmatch-1]` 包含对应子表达式的捕获区间（若有）
    - 未参与匹配的子表达式区间为 `{-1, -1}`
  - 若编译时设置了 `REG_NOSUB` 或 `nmatch == 0`：匹配结果不写入 `pmatch`
- **后置条件 (Case 2: 无匹配)**:
  - 返回值 = `REG_NOMATCH` (= 1)
  - `pmatch` 内容未定义（不应使用）
- **后置条件 (Case 3: 内存不足)**:
  - 返回值 = `REG_ESPACE` (= 12)
  - `pmatch` 内容未定义
- **系统算法（分派逻辑）**:
  1. 从 `preg` 的内部 opaque 字段提取 `tre_tnfa_t *tnfa`
  2. 若编译时指定了 `REG_NOSUB`，强制 `nmatch = 0`（即使调用方传入了非零值）
  3. 若需要捕获组信息（`num_tags > 0 && nmatch > 0`），分配标签数组
  4. 分派匹配引擎：
     - `tnfa->have_backrefs` 为真 → `tre_tnfa_run_backtrack`
     - 否则 → `tre_tnfa_run_parallel`
  5. 若匹配成功 (`REG_OK`)，调用 `tre_fill_pmatch` 将标签值转换为 `regmatch_t` 格式
  6. 释放标签数组，返回状态码
- **线程安全**: 是。该函数在栈上分配所有临时数据结构或通过线程安全的 `malloc` 获取，不修改全局状态或共享 `preg`（只读）。
- **POSIX 符合性**: 完全实现 POSIX.1-2001 的 `regexec()` 语义，包括左最长匹配规则和 `REG_NOTBOL`/`REG_NOTEOL` 标志。

---

## 跨文件依赖说明

| 依赖符号 | 类型 | 来源文件 | 说明 |
|---------|------|---------|------|
| `tre_tnfa_t` | 结构体 | `tre.h` | TNFA 核心数据结构，由 `regcomp` 构造 |
| `tre_tnfa_transition_t` | 结构体 | `tre.h` | TNFA 状态转换 |
| `tre_submatch_data_t` | 结构体 | `tre.h` | 子匹配元数据 |
| `tre_tag_direction_t` | 枚举 | `tre.h` | 标签方向枚举 |
| `tre_cint_t` | typedef (`wint_t`) | `tre.h` | 宽字符整型 |
| `tre_ctype_t` | typedef (`wctype_t`) | `tre.h` | 宽字符分类类型 |
| `tre_mem_new_impl` | 函数 (`hidden`) | `tre-mem.c` | 内存分配器创建，详见 tre-mem.c spec |
| `tre_mem_alloc_impl` | 函数 (`hidden`) | `tre-mem.c` | 内存分配器分配，详见 tre-mem.c spec |
| `tre_mem_destroy` | 函数 (`hidden`) | `tre-mem.c` | 内存分配器销毁，详见 tre-mem.c spec |
| `ALIGN` | 宏 | `tre.h` | 指针对齐计算 |
| `MB_LEN_MAX` | 宏 | `<limits.h>` | 多字节字符最大字节数 |
| `mbtowc` | 函数 | libc (`<wchar.h>`) | 多字节到宽字符转换 |
| `iswalnum`/`towlower`/`towupper`/`iswctype` | 函数 | libc (`<wctype.h>`) | 宽字符分类和转换 |
| `calloc`/`free`/`memset`/`strncmp` | 函数 | libc | 标准 C 库函数 |
| `regex_t`/`regmatch_t`/`regoff_t` | 类型 | `<regex.h>` | POSIX 正则类型 |
