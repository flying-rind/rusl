# tre.h 规约

> TRE 正则表达式引擎内部头文件。定义 TNFA（Tagged Nondeterministic Finite Automaton）的
> 数据结构、内存分配器抽象层、宽字符处理包装宏，以及用于正则表达式编译和匹配的所有
> 内部类型和常量。本文件是 musl libc 的正则表达式子系统（`regcomp`、`regexec` 等）
> 与 TRE 引擎核心之间的桥梁。

---

## 依赖图

```
tre.h (本文件) 包含:
  ├── <regex.h>   — POSIX 正则表达式公共 API（regex_t, regmatch_t 等）
  ├── <wchar.h>   — 宽字符类型定义（wchar_t, wint_t, wctype_t）
  └── <wctype.h>  — 宽字符分类/转换函数声明

tre.h 被以下文件引用:
  ├── regcomp.c   — 正则表达式编译（AST 构造 → TNFA 构造）
  ├── regexec.c   — 正则表达式匹配执行（TNFA 遍历）
  └── tre-mem.c   — TRE 内存分配器实现
```

---

## 第一部分：基础类型定义

### 1.1 TRE_REGEX_T_FIELD

```c
#define TRE_REGEX_T_FIELD __opaque
```

[Visibility]: Internal — musl 内部宏，POSIX 标准未定义，仅用于定义 `regex_t` 结构体内部字段名。

**意图**：为 `regex_t` 结构体中的 opaque 指针字段提供统一名称。musl 的 `regex_t` 定义为 `struct { ... void *__opaque; ... }`，本宏确保所有 TRE 内部代码通过同一标识符访问编译后的 TNFA 数据。

**不变量**：该宏展开后的标识符必须与 `<regex.h>` 中 `regex_t` 结构体的对应字段名严格一致。

---

### 1.2 reg_errcode_t

```c
typedef int reg_errcode_t;
```

[Visibility]: Internal — musl 内部类型别名，POSIX 标准未定义。外部用户应使用 `<regex.h>` 中定义的错误码常量（`REG_NOMATCH`、`REG_BADPAT` 等）。

**意图**：为正则表达式错误码提供语义化类型名，增强代码可读性。实际错误码值与 POSIX 的 `REG_*` 宏一致，均为 `int` 类型。

**不变量**：该类型始终与 `int` 兼容，可安全地在任何需要 `int` 的上下文中使用。

---

### 1.3 tre_char_t

```c
typedef wchar_t tre_char_t;
```

[Visibility]: Internal — musl 内部类型别名，POSIX 标准未定义。

**意图**：将 TRE 引擎的正则模式字符类型统一抽象为宽字符。musl 的 TRE 变体以宽字符为基础进行模式解析和匹配，与 POSIX 的 `wchar_t` 保持一致。

**不变量**：`tre_char_t` 与 `wchar_t` 完全等价，所有 `<wchar.h>` 函数均可直接作用于该类型的值。

---

### 1.4 tre_cint_t

```c
typedef wint_t tre_cint_t;
```

[Visibility]: Internal — musl 内部类型别名，POSIX 标准未定义。

**意图**：定义 TNFA 中字符范围值的类型。使用 `wint_t`（而非 `wchar_t`）以支持 `WEOF` 等超出 `wchar_t` 值域的特殊标记值。

**不变量**：该类型能容纳所有有效 Unicode 码点（0x00–0x10FFFF）外加特殊标记值。

---

### 1.5 tre_ctype_t

```c
typedef wctype_t tre_ctype_t;
```

[Visibility]: Internal — musl 内部类型别名，POSIX 标准未定义。

**意图**：为宽字符类别句柄提供 TRE 命名空间内的别名，用于 TNFA 中断言（assertion）的字符类别比较。

**不变量**：`tre_ctype_t` 与 `wctype_t` 完全等价，可直接传递给 `iswctype()`。

---

### 1.6 tre_tag_direction_t

```c
typedef enum {
  TRE_TAG_MINIMIZE = 0,
  TRE_TAG_MAXIMIZE = 1
} tre_tag_direction_t;
```

[Visibility]: Internal — musl TRE 内部枚举，POSIX 标准未定义。

**意图**：标记每个 submatch tag 的匹配策略——最小化匹配（non-greedy/懒惰）或最大化匹配（greedy/贪婪）。这直接对应于正则表达式中 `*?`、`+?` 等非贪婪量词与 `*`、`+` 等贪婪量词的语义差异。

**前置条件**：该枚举值仅用于 `tre_tnfa_t::tag_directions` 数组，数组长度等于 TNFA 中的 tag 总数。

**后置条件**：
- `TRE_TAG_MINIMIZE`：匹配引擎应尽可能短地匹配对应的子表达式
- `TRE_TAG_MAXIMIZE`：匹配引擎应尽可能长地匹配对应的子表达式（POSIX 最左最长规则）

---

## 第二部分：工具宏

### 2.1 NDEBUG

```c
#define NDEBUG
```

[Visibility]: Internal — musl 内部编译期开关，POSIX 标准未定义。

**意图**：无条件禁用 `<assert.h>` 断言检查。musl TRE 的正则表达式处理路径上，所有 `assert()` 在发布构建中被编译为空操作，以消除运行时开销。

**不变量**：必须位于任何 `#include <assert.h>` 之前（或该头文件被包含前定义），否则行为未定义。

---

### 2.2 DPRINT(msg)

```c
#define DPRINT(msg) do { } while(0)
```

[Visibility]: Internal — musl 内部调试宏，POSIX 标准未定义。

**意图**：提供一个可被重新定义为 `printf` 调用的调试打印桩（stub）。默认展开为空操作，在调试构建中可按需替换为实际日志输出。

**后置条件**：无论 `msg` 为何值，宏展开后不产生任何副作用（因 `do { } while(0)` 不执行任何代码）。

---

### 2.3 elementsof(x)

```c
#define elementsof(x)  ( sizeof(x) / sizeof(x[0]) )
```

[Visibility]: Internal — musl 内部工具宏，POSIX 标准未定义。

**意图**：编译期计算静态数组的元素个数。

**前置条件**：
- `x` 必须是一个**静态数组**（非指针），否则返回无意义的值
- `x[0]` 必须是有效类型以支持 `sizeof`

**后置条件**：返回类型为 `size_t` 的编译期常量，表示数组 `x` 中的元素个数。

---

### 2.4 ALIGN(ptr, type)

```c
#define ALIGN(ptr, type) \
  ((((long)ptr) % sizeof(type)) \
   ? (sizeof(type) - (((long)ptr) % sizeof(type))) \
   : 0)
```

[Visibility]: Internal — musl 内部工具宏，POSIX 标准未定义。

**意图**：计算将指针 `ptr` 对齐到 `type` 所需边界所需的字节偏移量。用于内存分配器中确保返回的指针满足特定类型的对齐要求。

**前置条件**：
- `ptr` 是 `char *` 类型或可安全转型为 `long` 的指针
- `type` 是完整的类型名

**后置条件**：
- 返回 `0` 到 `sizeof(type) - 1` 之间的值
- `(char *)ptr + ALIGN(ptr, type)` 满足 `type` 的对齐要求
- 若 `ptr` 已对齐，返回 `0`

---

### 2.5 MAX / MIN

```c
#define MAX(a, b) (((a) >= (b)) ? (a) : (b))
#define MIN(a, b) (((a) <= (b)) ? (a) : (b))
```

[Visibility]: Internal — musl 内部工具宏，POSIX 标准未定义。

**意图**：返回两个值的最大值/最小值。先 `#undef` 以确保覆盖任何系统预定义的同名宏。

**前置条件**：`a` 和 `b` 必须支持 `>=` 和 `<=` 运算符。

**后置条件**：
- 副作用安全：每个参数仅被求值一次（通过 `?:` 而非函数调用实现）

---

## 第三部分：宽字符包装宏

### 3.1 tre_mbrtowc

```c
#define tre_mbrtowc(pwc, s, n, ps) (mbtowc((pwc), (s), (n)))
```

[Visibility]: Internal — musl 内部包装宏，POSIX 标准未定义。

**意图**：将多字节到宽字符的转换统一映射到 `mbtowc()`（musl 内部的 `mbrtowc` 简化版本）。在 TRE 引擎的宽字符模式中，该宏用于将输入字符串的每个多字节序列解码为 `tre_cint_t`。

**前置条件**：
- `pwc` 指向有效的 `wchar_t` 存储位置（可为 `NULL`，此时仅计算字节数）
- `s` 指向有效的多字节字符串
- `n` 为 `s` 的可用字节数

**后置条件**：行为与 `mbtowc(pwc, s, n)` 完全一致。

---

### 3.2 TRE_CHAR_MAX

```c
#define TRE_CHAR_MAX 0x10ffff
```

[Visibility]: Internal — musl 内部常量，POSIX 标准未定义。

**意图**：定义 TNFA 中字符值的最大有效范围，即 Unicode 最大合法码点 U+10FFFF。

**不变量**：任何 `tre_cint_t` 值若大于 `TRE_CHAR_MAX`，则被视为特殊标记（如 `EMPTY`、`ASSERTION`、`TAG`、`BACKREF`），而非实际字符。

---

### 3.3 宽字符分类宏

```c
#define tre_isalnum  iswalnum
#define tre_isalpha  iswalpha
#define tre_isblank  iswblank
#define tre_iscntrl  iswcntrl
#define tre_isdigit  iswdigit
#define tre_isgraph  iswgraph
#define tre_islower  iswlower
#define tre_isprint  iswprint
#define tre_ispunct  iswpunct
#define tre_isspace  iswspace
#define tre_isupper  iswupper
#define tre_isxdigit iswxdigit
```

[Visibility]: Internal — musl 内部包装宏，POSIX 标准未定义。每个宏直接映射到对应的 POSIX 标准 `<wctype.h>` 宽字符分类函数。

**意图**：将 TRE 引擎的字符分类操作统一通过 `tre_` 前缀调用，提供一层薄抽象。这允许在非宽字符构建中替换为单字节版本，或注入自定义分类逻辑。

**不变量**：
- 每个 `tre_is*` 宏的行为与对应 `isw*` 函数完全相同
- 所有宏为纯函数，无副作用

---

### 3.4 宽字符转换与字符串宏

```c
#define tre_tolower towlower
#define tre_toupper towupper
#define tre_strlen  wcslen
```

[Visibility]: Internal — musl 内部包装宏，POSIX 标准未定义。

**意图**：与分类宏同理，为大小写转换和宽字符串长度计算提供 TRE 命名空间下的薄包装。

**不变量**：
- `tre_tolower(c)` 行为与 `towlower(c)` 相同
- `tre_toupper(c)` 行为与 `towupper(c)` 相同
- `tre_strlen(s)` 行为与 `wcslen(s)` 相同，`s` 必须以 `L'\0'` 结尾

---

### 3.5 tre_isctype / tre_ctype

```c
#define tre_isctype iswctype
#define tre_ctype   wctype
```

[Visibility]: Internal — musl 内部包装宏，POSIX 标准未定义。

**意图**：为泛型字符类别测试（`iswctype`）和类别句柄获取（`wctype`）提供 TRE 命名空间下的映射。

**不变量**：
- `tre_isctype(wc, desc)` 行为与 `iswctype(wc, desc)` 相同
- `tre_ctype(name)` 行为与 `wctype(name)` 相同

---

## 第四部分：TNFA 数据结构

### 4.1 tre_tnfa_transition_t — TNFA 转移

```c
typedef struct tnfa_transition tre_tnfa_transition_t;

struct tnfa_transition {
  tre_cint_t code_min;
  tre_cint_t code_max;
  tre_tnfa_transition_t *state;
  int state_id;
  int *tags;
  int assertions;
  union {
    tre_ctype_t class;
    int backref;
  } u;
  tre_ctype_t *neg_classes;
};
```

[Visibility]: Internal — musl TRE 内部数据结构，POSIX 标准未定义。

**意图**：TNFA 的单一状态转移边。每个 TNFA 状态由多条 `tre_tnfa_transition_t` 构成的数组表示，以 `state == NULL` 的转移作为数组终止标记。该结构体同时编码了字符匹配条件和转移后触发的动作（tag 记录、断言检查）。

**字段语义**：

| 字段 | 语义 |
|------|------|
| `code_min` / `code_max` | 该转移接受的字符范围（闭区间）。若 `code_min > TRE_CHAR_MAX`，则非普通字符转移，而是特殊 AST 节点（如 EMPTY、ASSERTION、TAG、BACKREF） |
| `state` | 目标状态的转移数组首地址。`NULL` 表示转移数组终止 |
| `state_id` | 目标状态的数字 ID，用于调试和 tag 计算 |
| `tags` | 以 -1 结尾的 tag 编号数组。转移发生时，这些 tag 被记录为当前匹配位置。`NULL` 表示无 tag |
| `assertions` | 位掩码，指示该转移附带的条件断言类型（参见 4.2 节） |
| `u.class` | 当 `assertions` 包含 `ASSERT_CHAR_CLASS` 时，指定要匹配的字符类别 |
| `u.backref` | 当 `assertions` 包含 `ASSERT_BACKREF` 时，指定反向引用的编号 |
| `neg_classes` | 当 `assertions` 包含 `ASSERT_CHAR_CLASS_NEG` 时，指向以 0 结尾的字符类别排除列表 |

**不变量**：
- TNFA 状态转移数组必须总是以 `state == NULL` 的元素结尾
- `code_min <= code_max`（对于普通字符转移）
- `tags` 若为非 NULL 则必须包含有效的 tag ID 并以 -1 结束
- 指针 `state` 为 NULL 仅用于终止标记，非 NULL 时指向有效的 `tre_tnfa_transition_t` 数组

---

### 4.2 断言位掩码常量

```c
#define ASSERT_AT_BOL          1    /* 行首 */
#define ASSERT_AT_EOL          2    /* 行尾 */
#define ASSERT_CHAR_CLASS      4    /* 字符类别匹配（正向） */
#define ASSERT_CHAR_CLASS_NEG  8    /* 字符类别匹配（反向） */
#define ASSERT_AT_BOW         16    /* 词首 */
#define ASSERT_AT_EOW         32    /* 词尾 */
#define ASSERT_AT_WB          64    /* 词边界 */
#define ASSERT_AT_WB_NEG     128    /* 非词边界 */
#define ASSERT_BACKREF       256    /* 反向引用 */
#define ASSERT_LAST          256    /* 最后一个断言编号（用于迭代边界） */
```

[Visibility]: Internal — musl TRE 内部常量，POSIX 标准未定义。

**意图**：定义 TNFA 转移中 `assertions` 字段的位掩码值。断言是零宽度的匹配条件——它们不消耗输入字符，仅在转移时检查当前位置是否满足特定上下文条件。

**断言语义**：

| 断言 | 语义 |
|------|------|
| `ASSERT_AT_BOL` | 当前位置是行首（考虑 `REG_NOTBOL` 和 `REG_NEWLINE` 标志） |
| `ASSERT_AT_EOL` | 当前位置是行尾（考虑 `REG_NOTEOL` 和 `REG_NEWLINE` 标志） |
| `ASSERT_CHAR_CLASS` | 当前字符属于 `u.class` 指定的字符类别 |
| `ASSERT_CHAR_CLASS_NEG` | 当前字符不属于 `neg_classes` 列表中任何字符类别 |
| `ASSERT_AT_BOW` | 当前位置是词首（前一字符非单词字符、当前字符是单词字符） |
| `ASSERT_AT_EOW` | 当前位置是词尾（前一字符是单词字符、当前字符非单词字符） |
| `ASSERT_AT_WB` | 当前位置是词边界（前后字符的单词属性不同） |
| `ASSERT_AT_WB_NEG` | 当前位置不是词边界（前后字符的单词属性相同，或位于字符串首/尾） |
| `ASSERT_BACKREF` | 检查当前子串是否与 `u.backref` 指定的已捕获子串匹配 |

**不变量**：
- `assertions` 字段可同时设置多个位（通过按位或组合），但 `ASSERT_CHAR_CLASS` 和 `ASSERT_CHAR_CLASS_NEG` 互斥
- 所有值均为 2 的幂，保证按位操作的语义正确性

---

### 4.3 tre_submatch_data_t — 子匹配数据

```c
struct tre_submatch_data {
  int so_tag;
  int eo_tag;
  int *parents;
};

typedef struct tre_submatch_data tre_submatch_data_t;
```

[Visibility]: Internal — musl TRE 内部数据结构，POSIX 标准未定义。

**意图**：为每个子表达式（捕获组）描述如何从 tag 值计算出 `regmatch_t` 中的 `rm_so`（起始偏移）和 `rm_eo`（结束偏移）。每个 submatch 关联两个 tag——一个标记匹配开始，一个标记匹配结束。

**字段语义**：

| 字段 | 语义 |
|------|------|
| `so_tag` | 提供 `rm_so` 值的 tag 编号。匹配引擎在执行过程中将该 tag 的值填入 `rm_so` |
| `eo_tag` | 提供 `rm_eo` 值的 tag 编号。匹配引擎在执行过程中将该 tag 的值填入 `rm_eo` |
| `parents` | 以 0 结尾的 submatch 编号数组，记录当前 submatch 被嵌套在哪些 submatch 中 |

**不变量**：
- `so_tag` 和 `eo_tag` 为有效的 tag 编号（在 `tre_tnfa_t::num_tags` 范围内）
- `parents` 若非 NULL 则必须以 0 结尾
- 每个 submatch 对应 `tre_submatch_data_t` 数组中的一个元素，数组索引即 submatch 编号

---

### 4.4 tre_tnfa_t — TNFA 定义

```c
typedef struct tnfa tre_tnfa_t;

struct tnfa {
  tre_tnfa_transition_t *transitions;
  unsigned int num_transitions;
  tre_tnfa_transition_t *initial;
  tre_tnfa_transition_t *final;
  tre_submatch_data_t *submatch_data;
  char *firstpos_chars;
  int first_char;
  unsigned int num_submatches;
  tre_tag_direction_t *tag_directions;
  int *minimal_tags;
  int num_tags;
  int num_minimals;
  int end_tag;
  int num_states;
  int cflags;
  int have_backrefs;
  int have_approx;
};
```

[Visibility]: Internal — musl TRE 内部核心数据结构，POSIX 标准未定义。外部用户仅通过 `regex_t` 中的 opaque 指针（`__opaque`）间接持有，不应直接访问其字段。

**意图**：TRE 编译产物的顶层结构，包含完整编译后的 TNFA 及其元数据。`regcomp()` 的编译结果存储于此结构，`regexec()` 的匹配过程以该结构为输入。这是所有 TRE 正则表达式操作的核心数据对象。

**字段语义**：

| 字段 | 类型 | 语义 |
|------|------|------|
| `transitions` | `tre_tnfa_transition_t *` | 指向所有转移边的扁平数组 |
| `num_transitions` | `unsigned int` | 转移边的总数量 |
| `initial` | `tre_tnfa_transition_t *` | 指向初始状态的转移数组 |
| `final` | `tre_tnfa_transition_t *` | 指向接受（终止）状态的转移数组 |
| `submatch_data` | `tre_submatch_data_t *` | submatch 元数据数组，长度等于 `num_submatches` |
| `firstpos_chars` | `char *` | 位图（256-bit），标记正则表达式可能匹配的**首字符**集合，用于快速排除不匹配的输入 |
| `first_char` | `int` | 若正则表达式以单个确定的字符开头，存储该字符值；否则为负值 |
| `num_submatches` | `unsigned int` | 子表达式（捕获组）总数，包括第 0 号（整体匹配） |
| `tag_directions` | `tre_tag_direction_t *` | 每个 tag 的匹配方向（贪婪/非贪婪），长度等于 `num_tags` |
| `minimal_tags` | `int *` | 最小化匹配的 tag 编号列表，以 -1 结尾 |
| `num_tags` | `int` | tag 总数 |
| `num_minimals` | `int` | 最小化匹配的 tag 数量 |
| `end_tag` | `int` | 标记整体匹配结束的 tag 编号（第 0 号 sub-match 的结束 tag） |
| `num_states` | `int` | TNFA 状态总数 |
| `cflags` | `int` | 编译标志（`REG_ICASE`、`REG_NEWLINE` 等），匹配时需要参考 |
| `have_backrefs` | `int` | 布尔值，若正则表达式包含反向引用则为真（影响匹配算法的复杂度） |
| `have_approx` | `int` | 布尔值，若使用近似匹配则为真（musl 默认构建中通常为假） |

**系统算法**（TNFA 遍历）：
匹配引擎使用 Thompson NFA 模拟算法在 `regexec.c` 中实现。该算法维护一组活跃状态，逐字符推进输入并计算可达状态集。submatch 值通过 tag 记录机制确定：当转移边上的 tag 被触发时，当前输入偏移被记录；在匹配成功后，根据各 tag 的方向（贪婪/非贪婪）及优先级规则确定最终的 `rm_so`/`rm_eo` 值。

**不变量**：
- `initial` 和 `final` 必须指向 `transitions` 数组内的有效位置
- `num_submatches >= 1`（第 0 号始终存在，对应整体匹配）
- `submatch_data` 数组的长度为 `num_submatches`
- `tag_directions` 数组的长度为 `num_tags`
- `firstpos_chars` 的长度为 32 字节（256 位）
- 若 `have_backrefs` 为真，匹配引擎必须启用反向引用解析路径
- TNFA 对象由 TRE 内存分配器分配，其生命周期由 `tre_mem_t` 管理

---

## 第五部分：TRE 内存分配器

### 5.1 TRE_MEM_BLOCK_SIZE

```c
#define TRE_MEM_BLOCK_SIZE 1024
```

[Visibility]: Internal — musl 内部常量，POSIX 标准未定义。

**意图**：定义 TRE 内存分配器每次从系统申请的内存块默认大小（1KB）。较小的块大小适合正则表达式编译中大量小对象的分配模式，减少内部碎片。

**不变量**：该值为正且为 2 的幂（便于对齐）。

---

### 5.2 tre_list_t — 内存块链表节点

```c
typedef struct tre_list {
  void *data;
  struct tre_list *next;
} tre_list_t;
```

[Visibility]: Internal — musl 内部数据结构，POSIX 标准未定义。

**意图**：TRE 内存分配器内部使用的单向链表节点，用于跟踪已分配的内存块，以便在 `tre_mem_destroy()` 时统一释放。

**字段语义**：

| 字段 | 语义 |
|------|------|
| `data` | 指向从系统分配的内存块的指针 |
| `next` | 指向下一个链表节点，`NULL` 表示链表尾 |

**不变量**：`data` 始终非 NULL（链表仅包含有效的已分配块）。

---

### 5.3 tre_mem_struct / tre_mem_t — 内存分配器实例

```c
typedef struct tre_mem_struct {
  tre_list_t *blocks;
  tre_list_t *current;
  char *ptr;
  size_t n;
  int failed;
  void **provided;
} *tre_mem_t;
```

[Visibility]: Internal — musl 内部数据结构，POSIX 标准未定义。外部用户不应创建或操作此类型，它由 `regcomp()` 内部使用并随 `regex_t` 释放。

**意图**：TRE 的内存池分配器控制块。该分配器专为正则表达式编译优化——编译过程中产生大量生命周期相同的小对象（AST 节点、TNFA 转移边、tag 数组等），使用池分配器避免了频繁的 `malloc`/`free` 调用，且支持一次性释放所有内存。

**字段语义**：

| 字段 | 类型 | 语义 |
|------|------|------|
| `blocks` | `tre_list_t *` | 已分配的内存块链表头 |
| `current` | `tre_list_t *` | 当前正在从中分配的内存块所在链表节点 |
| `ptr` | `char *` | 当前内存块中下一个空闲位置的指针 |
| `n` | `size_t` | 当前内存块中剩余的可用字节数 |
| `failed` | `int` | 失败标志。一旦内存分配失败（`malloc` 返回 NULL），该标志置 1，此后所有分配请求立即返回 NULL |
| `provided` | `void **` | 用于 alloca 模式：指向外部提供的内存块引用的指针 |

**系统算法**（池分配策略）：
1. 分配请求先检查当前块剩余空间是否足够。
2. 若足够，从当前块切出所需大小（含对齐填充），更新 `ptr` 和 `n`。
3. 若不足，分配新块：若请求大小超过 `TRE_MEM_BLOCK_SIZE` 的 8 倍，新块大小等于请求大小的 8 倍；否则默认为 `TRE_MEM_BLOCK_SIZE`。
4. 新块加入 `blocks` 链表，成为新的 `current`。
5. 所有内存通过 `tre_mem_destroy()` 遍历链表一次性 `free`。

**不变量**：
- `ptr` 指向 `current->data` 内的偏移位置
- `n <= TRE_MEM_BLOCK_SIZE`（或对于大请求，`n <= size * 8`）
- `failed == 1` 意味着之前的某次分配已失败，此后分配器处于"降级模式"
- `blocks` 链表的所有 `data` 指针均唯一且来自 `malloc`

---

### 5.4 tre_mem_new_impl — 创建内存分配器

```c
hidden tre_mem_t tre_mem_new_impl(int provided, void *provided_block);
```

[Visibility]: Internal — `hidden` 可见性，不对外导出。musl 内部函数，POSIX 标准未定义。

**意图**：创建并初始化一个新的 TRE 内存分配器实例。支持两种模式：(1) 从堆分配新实例 (`provided=0`)；(2) 使用外部提供的内存块 (`provided=1`)，通常由 `alloca()` 在栈上分配。

**前置条件**：
- 若 `provided == 0`：`provided_block` 应为 `NULL`
- 若 `provided == 1`：`provided_block` 指向至少 `sizeof(struct tre_mem_struct)` 字节的有效内存

**后置条件**：
- Case 1（成功）：返回指向已初始化（零填充）的 `tre_mem_struct` 的指针，所有字段归零
- Case 2（失败）：返回 `NULL`，若 `provided == 0` 说明底层 `calloc` 失败

**不变量**：返回的分配器实例初始状态满足：`blocks == NULL`，`ptr == NULL`，`n == 0`，`failed == 0`。

---

### 5.5 tre_mem_destroy — 销毁内存分配器

```c
hidden void tre_mem_destroy(tre_mem_t mem);
```

[Visibility]: Internal — `hidden` 可见性，不对外导出。musl 内部函数，POSIX 标准未定义。

**意图**：释放分配器管理的所有内存（包括所有已分配块和分配器控制结构本身）。

**前置条件**：
- `mem` 必须是通过 `tre_mem_new()` 或 `tre_mem_newa()` 创建的有效分配器实例
- 或 `mem` 为 `NULL`（函数应在调用前检查）

**后置条件**：
- `mem` 指向的所有块链表节点及数据块均被 `free`
- `mem` 控制结构本身被 `free`
- `mem` 指针变为无效（悬垂指针），调用方不得再使用

**不变量**：调用后无内存泄漏——所有通过该分配器分配的内存均被释放。

---

### 5.6 tre_mem_alloc_impl — 分配内存

```c
hidden void *tre_mem_alloc_impl(tre_mem_t mem, int provided, void *provided_block,
                                int zero, size_t size);
```

[Visibility]: Internal — `hidden` 可见性，不对外导出。musl 内部函数，POSIX 标准未定义。

**意图**：从 TRE 内存分配器中分配 `size` 字节的内存。支持可选的零初始化、外部提供块模式。

**前置条件**：
- `mem` 为有效的 `tre_mem_t` 实例
- 若 `mem->failed == 1`，函数将立即返回 `NULL`
- `size > 0`
- 若 `provided == 1` 且当前块空间不足：`provided_block` 必须非 NULL（否则设置 `mem->failed = 1`）

**后置条件**：
- Case 1（成功）：返回指向 `size` 字节内存的指针，指针满足 `long` 类型的对齐要求；若 `zero != 0`，内存已零填充
- Case 2（失败）：返回 `NULL`，`mem->failed` 被设置为 `1`

**系统算法**：参见 5.3 节的池分配策略描述。

**不变量**：
- 分配的内存位于由 `mem` 管理的块中，释放 `mem` 时自动回收
- 返回的指针始终满足 `long` 的对齐要求
- `failed` 标志一旦置 1 则不再清除

---

### 5.7 内存分配器便捷宏

```c
#define tre_mem_new()            tre_mem_new_impl(0, NULL)
#define tre_mem_alloc(mem, size) tre_mem_alloc_impl(mem, 0, NULL, 0, size)
#define tre_mem_calloc(mem, size) tre_mem_alloc_impl(mem, 0, NULL, 1, size)
```

[Visibility]: Internal — musl 内部宏，POSIX 标准未定义。

**意图**：为常见分配模式提供简洁接口。

| 宏 | 等价调用 | 用途 |
|----|---------|------|
| `tre_mem_new()` | `tre_mem_new_impl(0, NULL)` | 从堆创建新分配器 |
| `tre_mem_alloc(mem, size)` | `tre_mem_alloc_impl(mem, 0, NULL, 0, size)` | 从分配器分配 `size` 字节（未初始化） |
| `tre_mem_calloc(mem, size)` | `tre_mem_alloc_impl(mem, 0, NULL, 1, size)` | 从分配器分配 `size` 字节（零初始化） |

**不变量**：每个宏的行为与对应的 `*_impl` 调用完全等价。

---

### 5.8 alloca 模式宏（条件编译）

```c
#ifdef TRE_USE_ALLOCA
#define tre_mem_newa() \
  tre_mem_new_impl(1, alloca(sizeof(struct tre_mem_struct)))

#define tre_mem_alloca(mem, size) \
  ((mem)->n >= (size) \
   ? tre_mem_alloc_impl((mem), 1, NULL, 0, (size)) \
   : tre_mem_alloc_impl((mem), 1, alloca(TRE_MEM_BLOCK_SIZE), 0, (size)))
#endif
```

[Visibility]: Internal — musl 内部宏，POSIX 标准未定义。仅在 `TRE_USE_ALLOCA` 宏被定义时生效（musl 默认构建中**未启用**）。

**意图**：提供基于栈分配的 TRE 内存分配器变体，消除编译过程中的堆分配开销。所有分配器和数据块通过 `alloca()` 在栈上分配，函数返回时自动回收——无需调用 `tre_mem_destroy()`。

**前置条件**：
- `TRE_USE_ALLOCA` 必须在编译时定义
- 栈空间必须足够容纳所有分配（编译器无栈大小保证，由调用方评估风险）

**后置条件**：
- `tre_mem_newa()`：返回在栈上分配的分配器实例
- `tre_mem_alloca(mem, size)`：从栈上分配 `size` 字节；若当前块不足，自动分配新的 `TRE_MEM_BLOCK_SIZE` 大小的栈块

**不变量**：
- 所有通过 `tre_mem_newa()` 和 `tre_mem_alloca()` 分配的内存均在函数返回时由栈帧回收自动释放
- 不得对 alloca 模式分配器调用 `tre_mem_destroy()`

---

## 第六部分：系统分配器别名

```c
#define xmalloc  malloc
#define xcalloc  calloc
#define xfree    free
#define xrealloc realloc
```

[Visibility]: Internal — musl 内部宏别名，POSIX 标准未定义。

**意图**：将 TRE 引擎中的系统内存分配调用通过 `x` 前缀别名统一。在原始 TRE 库中，`xmalloc` 等函数包含内存不足时的错误处理（如打印错误并 `abort()`）；musl 中直接映射到标准 `malloc`/`free`，由调用方或上层逻辑处理分配失败。

**不变量**：
- `xmalloc(size)` 行为与 `malloc(size)` 完全相同
- `xcalloc(n, size)` 行为与 `calloc(n, size)` 完全相同
- `xfree(ptr)` 行为与 `free(ptr)` 完全相同
- `xrealloc(ptr, size)` 行为与 `realloc(ptr, size)` 完全相同

---

## 附录 A：符号导出状态汇总

| 符号 | 类别 | 导出状态 |
|------|------|---------|
| `TRE_REGEX_T_FIELD` | 宏 | Internal — musl 内部，定义 `regex_t` opaque 字段名 |
| `NDEBUG` | 宏 | Internal — 禁用断言，`#include` 前定义 |
| `DPRINT` | 宏 | Internal — 调试打印桩（空操作） |
| `elementsof` | 宏 | Internal — 静态数组元素计数 |
| `ALIGN` | 宏 | Internal — 指针对齐偏移计算 |
| `MAX` / `MIN` | 宏 | Internal — 最大/最小值 |
| `tre_mbrtowc` | 宏 | Internal — 多字节到宽字符转换 |
| `TRE_CHAR_MAX` | 宏 | Internal — Unicode 最大码点常量 |
| `tre_isalnum` 等 12 个 | 宏 | Internal — 宽字符分类包装 |
| `tre_tolower` / `tre_toupper` | 宏 | Internal — 宽字符大小写转换包装 |
| `tre_strlen` | 宏 | Internal — 宽字符串长度包装 |
| `tre_isctype` / `tre_ctype` | 宏 | Internal — 泛型字符类别包装 |
| `reg_errcode_t` | typedef | Internal — 错误码类型别名 |
| `tre_char_t` | typedef | Internal — 字符类型别名 |
| `tre_cint_t` | typedef | Internal — 字符整型类型别名 |
| `tre_ctype_t` | typedef | Internal — 字符类别类型别名 |
| `tre_tag_direction_t` | enum | Internal — tag 匹配方向枚举 |
| `tre_tnfa_transition_t` | struct | Internal — TNFA 转移边 |
| `ASSERT_AT_BOL` 等 10 个 | 宏 | Internal — TNFA 断言位掩码 |
| `tre_submatch_data_t` | struct | Internal — submatch 元数据 |
| `tre_tnfa_t` | struct | Internal — TNFA 顶层结构 |
| `TRE_MEM_BLOCK_SIZE` | 宏 | Internal — 分配器块大小 |
| `tre_list_t` | struct | Internal — 内存块链表节点 |
| `tre_mem_struct` / `tre_mem_t` | struct/typedef | Internal — 内存分配器控制块 |
| `tre_mem_new_impl` | 函数 | Internal — `hidden`，创建内存分配器 |
| `tre_mem_alloc_impl` | 函数 | Internal — `hidden`，分配内存 |
| `tre_mem_destroy` | 函数 | Internal — `hidden`，销毁内存分配器 |
| `tre_mem_new` / `tre_mem_alloc` / `tre_mem_calloc` | 宏 | Internal — 内存分配便捷宏 |
| `tre_mem_newa` / `tre_mem_alloca` | 宏 | Internal — alloca 模式宏（条件编译） |
| `xmalloc` / `xcalloc` / `xfree` / `xrealloc` | 宏 | Internal — 系统分配器别名 |

---

## 附录 B：与其他模块的关系

| 相关文件 | 关系 |
|----------|------|
| `tre-mem.c` | 实现 `tre_mem_new_impl`、`tre_mem_alloc_impl`、`tre_mem_destroy`（参见 tre-mem.c spec） |
| `regcomp.c` | 使用者：通过 `tre_mem_t` 分配编译期间临时数据；构造 `tre_tnfa_t` 作为编译输出（参见 regcomp.c spec） |
| `regexec.c` | 使用者：遍历 `tre_tnfa_t` 执行匹配；解释断言位掩码和 tag 记录（参见 regexec.c spec） |
| `<regex.h>` | POSIX 公共头文件：定义 `regex_t`（包含 `__opaque` 字段指向 `tre_tnfa_t`）、`regmatch_t` 等公共类型 |

---

*本规约基于 musl TRE 内部头文件生成，覆盖所有声明的符号及其递归依赖。所有标记为 Internal 的符号均为 musl TRE 正则表达式引擎内部实现细节，未被 POSIX 或 ISO C 标准定义，外部用户程序不应直接依赖。*
