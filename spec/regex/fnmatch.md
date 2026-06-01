# fnmatch.c 规约

## 概述

本文件实现了 POSIX `fnmatch()` 函数，采用 Rich Felker 设计的 "Sea of Stars" 算法（2012年4月）。其核心思想是将模式分解为头部（head）、可选的尾部（tail），以及由 `*` 分隔的中间组件集合（"sea of stars"）。在匹配完头部和尾部后，中间组件按顺序从上次匹配位置之后搜索首次出现处。此算法避免了在匹配失败时必须预先计算字符串长度。

---

## 依赖图

```
fnmatch (Public)
  ├── pat_next (Internal)       — 模式 token 读取器
  ├── fnmatch_internal (Internal) — 核心匹配引擎
  │     ├── pat_next (Internal)
  │     ├── str_next (Internal)  — 字符串字符读取器
  │     ├── casefold (Internal)  — 大小写折叠
  │     ├── match_bracket (Internal) — 方括号表达式匹配
  │     └── strnlen (External, <string.h>)
  └── strnlen (External, <string.h>)

外部依赖（不在此生成规约）:
  - strnlen, memcpy (<string.h>)
  - mbtowc (<wchar.h>)
  - towupper, towlower, iswctype, wctype (<wctype.h>)
  - MB_CUR_MAX (<stdlib.h>, 由 locale_impl.h 重定义)
  - FNM_* 标志宏 (<fnmatch.h>)
```

---

## 内部宏定义

```c
#define END          0
#define UNMATCHABLE -2
#define BRACKET     -3
#define QUESTION    -4
#define STAR        -5
```

[Visibility]: Internal (不导出) — musl fnmatch 内部实现使用的 token 类型编码，POSIX 标准未定义

| 宏 | 值 | 含义 |
|----|-----|------|
| `END` | 0 | 模式/字符串结束，或空字符 |
| `UNMATCHABLE` | -2 | 非法多字节序列，无法匹配任何字符 |
| `BRACKET` | -3 | 方括号表达式 `[...]` |
| `QUESTION` | -4 | 问号 `?` 通配符 |
| `STAR` | -5 | 星号 `*` 通配符 |

`pat_next` 返回负值时表示特殊 token 类型；返回非负值时表示字面字符（Unicode 码点或单字节 ASCII）。

---

## str_next (内部函数)

```c
static int str_next(const char *str, size_t n, size_t *step)
```

[Visibility]: Internal (不导出) — musl fnmatch 内部辅助函数，POSIX/C 标准未定义

### 意图 (Intent)

从字符串 `str` 中读取下一个字符（可能是多字节 UTF-8 字符），并告知调用者消耗了多少字节。此函数是 "Sea of Stars" 算法中所有字符串遍历的字符级原子操作。

### 前置条件

- `str` 必须指向有效的内存区域
- `n` 表示 `str` 中可用的最大字节数（含终止空字符，若已计算长度）
- `step` 必须指向有效的 `size_t` 变量
- `n >= 0`

### 后置条件

**Case 1: `n == 0`（无可用字符）**
- `*step = 0`
- 返回 `0`（视为 END）
- 副作用：仅修改 `*step`

**Case 2: `str[0] >= 128U`（多字节 UTF-8 字符）**
- 调用 `mbtowc(&wc, str, n)` 解码多字节字符
- 若 `mbtowc` 返回 `k < 0`（非法序列）：
  - `*step = 1`（跳过 1 个字节）
  - 返回 `-1`（不可匹配）
- 若解码成功：
  - `*step = k`（多字节字符占用的字节数）
  - 返回对应宽字符 `wc` 的值

**Case 3: `str[0] < 128U`（单字节 ASCII）**
- `*step = 1`
- 返回 `str[0]`（直接当作字符值）

### 不变量 (Invariants)

- `*step <= n`（步长永不超过剩余长度）
- 返回值为 `-1` 时，用于指示需要跳过非法序列字符
- 返回值 `> 0` 时，为有效的 Unicode 码点（或单字节 ASCII 值）

---

## pat_next (内部函数)

```c
static int pat_next(const char *pat, size_t m, size_t *step, int flags)
```

[Visibility]: Internal (不导出) — musl fnmatch 内部辅助函数，POSIX/C 标准未定义

### 意图 (Intent)

从模式字符串 `pat` 中读取下一个 token。Token 可以是特殊通配符（`*`、`?`、`[...]`）、字面字符、或为非法序列标记。此函数是模式解析的原子操作，同时处理转义字符（根据 `FNM_NOESCAPE` 标志决定是否启用以 `\` 开头的转义）。

### 前置条件

- `pat` 必须指向有效的内存区域
- `m` 表示 `pat` 中可用的最大字节数（含终止空字符，若已计算长度）
- `step` 必须指向有效的 `size_t` 变量
- `flags` 包含 `FNM_NOESCAPE` 位以控制转义行为
- `m >= 0`

### 后置条件

**Case 1: `m == 0` 或 `*pat == '\0'`（模式结束）**
- `*step = 0`
- 返回 `END`

**Case 2: `pat[0] == '\\'` 且转义未禁用 (`!(flags & FNM_NOESCAPE)`) 且 `pat[1]` 非空**
- 跳过反斜杠，处理转义字符
- 若转义后的字符 `>= 128U`（多字节）：
  - 解码多字节字符
  - 解码失败 → `*step = 0`，返回 `UNMATCHABLE`
  - 解码成功 → `*step = k + esc`（k 为多字节字节数，esc=1 为反斜杠），返回宽字符值
- 若为单字节字符 → `*step = 1 + esc`（通常 `= 2`），返回该字符（如 `\\` 返回 `'\\'`）

**Case 3: `pat[0] == '['`（方括号表达式起点）**
- 扫描至匹配的 `]`，正确处理：
  - `[^...]` 和 `[!...]`（取反标记）
  - `[]...]` 和 `[^]...]`（`]` 作为首个字符被当作字面量）
  - `[:class:]`、`[.coll.]`、`[=equiv=]` 嵌套结构
  - 嵌套结构的结束条件为 `pat[k-1] == z && pat[k] == ']'`（z 为 `:`、`.` 或 `=`）
- 若找到匹配的 `]` → `*step = k+1`（整个方括号表达式的字节长度），返回 `BRACKET`
- 若未找到匹配的 `]`（`k == m` 或 `pat[k] == '\0'`）→ `*step = 1`，返回 `'['`（作为字面字符处理 — 此时方括号表达式不完整）

**Case 4: `pat[0] == '*'`**
- `*step = 1`
- 返回 `STAR`

**Case 5: `pat[0] == '?'`**
- `*step = 1`
- 返回 `QUESTION`

**Case 6: 多字节字面字符 (`>= 128U`)**
- 解码多字节字符
- 解码失败 → `*step = 0`，返回 `UNMATCHABLE`
- 解码成功 → `*step = k`（若有转义则为 `k + esc`），返回宽字符值

**Case 7: 单字节字面字符 (`< 128U`)**
- `*step = 1`（若有转义则加上 esc 后可能为 2）
- 返回 `pat[0]`

### 不变量 (Invariants)

- `*step` 永不超过 `m`（步长受剩余长度约束）
- 返回值 `>= 0` 表示字面字符；返回值 `< 0` 表示特殊 token
- `UNMATCHABLE` 返回值表示模式中存在非法多字节序列，此模式整体不可能匹配成功

---

## casefold (内部函数)

```c
static int casefold(int k)
```

[Visibility]: Internal (不导出) — musl fnmatch 内部辅助函数，POSIX/C 标准未定义

### 意图 (Intent)

对单个字符进行大小写折叠，用于 `FNM_CASEFOLD` 模式匹配。策略：先尝试 `towupper(k)`，若结果等于 `k`（即原字符已是大写或无大写形式），则尝试 `towlower(k)`。

### 前置条件

- `k` 为有效的字符码点（或 `EOF`）

### 后置条件

- 若 `towupper(k) != k`：返回 `towupper(k)`（转大写结果）
- 若 `towupper(k) == k`：返回 `towlower(k)`（转小写结果，对于无大小写区分的字符也返回自身）
- 始终返回一个有效字符码点

---

## match_bracket (内部函数)

```c
static int match_bracket(const char *p, int k, int kfold)
```

[Visibility]: Internal (不导出) — musl fnmatch 内部辅助函数，POSIX/C 标准未定义

### 意图 (Intent)

判断字符 `k`（或其大小写折叠形式 `kfold`）是否匹配方括号表达式 `[...]`。支持取反 `[^...]` / `[!...]`、字符范围 `a-z`、字符类 `[:class:]`、以及多字节字符。

### 前置条件

- `p` 指向方括号表达式的 `[` 字符之后（即 `p[-1] == '['`）
- `k` 为待匹配的字符码点
- `kfold` 为 `k` 的大小写折叠形式（若 `FNM_CASEFOLD` 未启用则 `kfold == k`）
- `*p` 必须是有效内存，且后方存在匹配的 `]`

### 后置条件

**Case 1: 取反模式 `p[0] == '^'` 或 `p[0] == '!'`**
- 设置 `inv = 1`，将 `p` 前移
- 若列表中任意元素匹配 `k` 或 `kfold`：返回 `!inv`（即 `0`，不匹配）
- 若 `*p == ']'`（右括号出现在取反符号后）被视为列表的终止而非字面 `]` — 注意：此处的逻辑是对 POSIX 标准的特化处理

**Case 2: `*p == ']'` 且 `k == ']'`**
- 将 `]` 视为列表字面成员
- 返回 `!inv`（即匹配成功）

**Case 3: `*p == '-'` 且 `k == '-'`**
- 将 `-` 视为列表字面成员
- 返回 `!inv`（即匹配成功）

**Case 4: 字符范围 `a-z`（`p[0] == '-' && p[1] != ']'`）**
- 读取范围终点 `wc2`
- 若 `wc <= wc2` 且 `k` 在 `[wc, wc2]` 范围内（或 `kfold` 在范围内）：返回 `!inv`（匹配）
- 判断范围包含关系：`(unsigned)(k - wc) <= wc2 - wc`
- 范围终点若为多字节字符且无法解码 → 返回 `0`（不匹配）

**Case 5: 字符类 `[:class:]`、`[.coll.]`、`[=equiv=]`**
- 对于 `[:class:]`（`z == ':'` 且类名字符串长度 `< 16`）：
  - 通过 `wctype(buf)` 获取字符类别
  - 若 `iswctype(k, <class>)` 或 `iswctype(kfold, <class>)`：返回 `!inv`（匹配）
- 对于 `[.coll.]` 和 `[=equiv=]`：当作不匹配处理（继续扫描）

**Case 6: 单字符或多字节字面匹配**
- 若 `*p < 128U`：`wc = (unsigned char)*p`
- 否则调用 `mbtowc(&wc, p, 4)` 解码 → 失败返回 `0`（不匹配）
- 若 `wc == k` 或 `wc == kfold`：返回 `!inv`（匹配）

**Case 7: 所有元素遍历完毕且无匹配**
- 返回 `inv`（无元素匹配时，取反模式返回 `1` 匹配，正常模式返回 `0` 不匹配）

### 不变量 (Invariants)

- 函数不会读取越过首个未转义的 `]` 之后
- 扫描过程中若遇到 `mbtowc` 失败（返回 `< 0`），函数返回 `0`（不匹配）

---

## fnmatch_internal (内部函数)

```c
static int fnmatch_internal(const char *pat, size_t m, const char *str, size_t n, int flags)
```

[Visibility]: Internal (不导出) — musl fnmatch 核心匹配引擎，POSIX/C 标准未定义

### 系统算法 (System Algorithm)

"Sea of Stars" 算法 — 将模式分解为头部、尾部和由 `*` 分隔的中间组件。算法分四个阶段：

1. **头部匹配 (Head Match)**：从模式开头匹配到第一个 `*`，若失配则立即返回 `FNM_NOMATCH`，无需知道字符串长度。此阶段是算法的关键优化。

2. **尾部收集 (Tail Collection)**：重新扫描整个模式，定位最后一个 `*` 及其后的所有字面字符/token，累计 `tailcnt`。

3. **尾部匹配 (Tail Match)**：从字符串末尾提取 `tailcnt` 个字符，与模式尾部逐 token 比较。若失配则返回 `FNM_NOMATCH`。

4. **星海匹配 (Sea of Stars)**：在头部和尾部之间，找到由 `*` 分隔的每个组件，按顺序在字符串中搜索其首次出现处。若某组件找不到匹配位置，则推进起始位置并重试。

### 前置条件

- `pat` 指向模式字符串，`m` 为模式长度（`-1` 表示待计算，将使用 `strnlen` 计算）
- `str` 指向待匹配字符串，`n` 为字符串长度（`-1` 表示待计算）
- `flags` 包含 `FNM_*` 标志位组合
- 调用者应确保不会传入含有不可修复 `UNMATCHABLE` 的模式（尽管函数内部会检测并返回 `FNM_NOMATCH`）

### 后置条件

**Case 1: 匹配成功**
- 返回 `0`

**Case 2: 匹配失败**
- 返回 `FNM_NOMATCH`（值为 `1`）

### 行为详细描述

#### 阶段 0: `FNM_PERIOD` 前缀检查

若 `flags & FNM_PERIOD`：
- 若 `*str == '.'` 且 `*pat != '.'`：返回 `FNM_NOMATCH`（前导句点必须被显式匹配）

#### 阶段 1: 头部匹配

循环调用 `pat_next` 读取模式 token，直到遇到第一个 `STAR`：
- 每个 token 必须与字符串的对应字符匹配（通过 `str_next` 读取）
- `BRACKET` token 委托给 `match_bracket`
- `QUESTION` token 匹配任意一个非空字符
- 字面 token 要求 `k == c` 或在 `FNM_CASEFOLD` 下 `kfold == c`
- 若字符串提前结束（`k <= 0`）而模式未结束（`c != END`）：返回 `FNM_NOMATCH`
- 若字符串和模式同时结束（`k <= 0 && c == END`）：返回 `0`（匹配）

#### 阶段 2: 尾部收集

在遇到第一个 `STAR` 后，重新扫描整个模式：
- 定位最后一个 `*` 的位置 `ptail`
- 累计 `tailcnt`（最后一个 `*` 后的字面 token 数量）

#### 阶段 3: 尾部匹配

- 计算字符串长度（若 `n == -1`）
- 从字符串末尾提取 `tailcnt` 个字符（正确处理 UTF-8：多字节字符的第二/后续字节特征为 `(byte - 0x80) < 0x40`）
- 从模式尾部 `ptail` 开始逐 token 与字符串尾部比较（逻辑同阶段 1）
- 失配时返回 `FNM_NOMATCH`

#### 阶段 4: 星海匹配

在 `pat`（头部之后）到 `endpat`（尾部之前）的区间中：
- 取出由 `*` 分隔的每个组件（从 `pat` 到下一个 `STAR` 或 `endpat`）
- 在字符串当前位置 `str` 到 `endstr` 中搜索该组件的首次匹配
- 若找到匹配，更新 `pat` 和 `str` 到匹配结束位置，继续下一组件
- 若未找到匹配，将字符串起始位置 `str` 前移一个字符：
  - 有效字符：`str += sinc`
  - 非法序列：逐个字节跳过，直至找到有效字符开头
  - 重新搜索同一组件
- 只有当 `pat >= endpat` 时才算匹配成功

### 不变量 (Invariants)

- `str <= endstr` 且 `pat <= endpat`（不越过已确定的边界）
- 尾部匹配阶段中 `endstr` 和 `endpat` 不会被修改（仅在阶段切换时计算）
- 星海阶段结束条件与尾部匹配阶段的边界划分保持一致

---

## fnmatch (对外导出)

```c
int fnmatch(const char *pat, const char *str, int flags)
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<fnmatch.h>` 声明

### 意图 (Intent)

测试字符串 `str` 是否与 shell 通配符模式 `pat` 匹配。支持 `*`（匹配零个或多个字符）、`?`（匹配单个字符）和 `[...]`（字符类/范围匹配）。通过 `flags` 参数控制匹配行为（如路径名模式、大小写折叠、转义禁用等）。

### 前置条件

- `pat` 和 `str` 必须是以空字符结尾的有效 C 字符串
- `flags` 是 `FNM_*` 标志的位或组合，可包含：
  - `FNM_PATHNAME` (0x1): 路径名模式 — `*` 和 `?` 不匹配 `/`；模式按 `/` 分段分别匹配
  - `FNM_NOESCAPE` (0x2): 禁用 `\` 转义
  - `FNM_PERIOD` (0x4): 前导句点 — 字符串的首字符若为 `.` 则必须被模式首字符显式匹配
  - `FNM_LEADING_DIR` (0x8): 前导目录 — 若字符串中存在 `/` 且在 `/` 位置匹配成功，则视为整体匹配成功
  - `FNM_CASEFOLD` (0x10): 大小写折叠 — 匹配时忽略大小写

### 后置条件

**Case 1: 匹配成功**
- 返回 `0`

**Case 2: 匹配失败**
- 返回 `FNM_NOMATCH` (值为 `1`)

**Case 3: 系统不支持**
- 理论上可返回 `FNM_NOSYS` (值为 `-1`)，但 musl 实现从不返回此值

### 行为详细描述

#### `FNM_PATHNAME` 模式（`flags & FNM_PATHNAME`）

采用逐路径段匹配策略：

1. 逐段扫描 `str` 和 `pat`：
   - 在字符串中找到下一个 `/` 的位置（`s` 扫描）
   - 在模式中找到下一个 `/` 的位置（`p` 扫描），使用 `pat_next` 读取
2. 对每一段调用 `fnmatch_internal(pat, p-pat, str, s-str, flags)` 进行段内匹配
3. 若某段匹配失败：返回 `FNM_NOMATCH`
4. 若 `c != *s`（模式终止字符 `END` 或 `/` 与字符串终止字符不同）：
   - 若 `*s == '\0'` 且 `flags & FNM_LEADING_DIR`：该段匹配成功时视为整体匹配成功
   - 否则返回 `FNM_NOMATCH`
5. 若模式段已结束（`c == END`）而字符串还有剩余：返回 `FNM_NOMATCH`（除非满足 LEADING_DIR 语义）
6. 所有段匹配完毕：返回 `0`

#### `FNM_LEADING_DIR` 模式（`flags & FNM_LEADING_DIR`，但无 `FNM_PATHNAME`）

- 扫描字符串中每个 `/` 位置
- 对字符串前缀（到 `/` 之前）调用 `fnmatch_internal` 尝试匹配
- 若任意前缀匹配成功：返回 `0`
- 若所有前缀都失配：最终尝试整个字符串匹配，返回 `fnmatch_internal(pat, -1, str, -1, flags)` 的结果

#### 普通模式（无 `FNM_PATHNAME` 和 `FNM_LEADING_DIR`）

- 直接调用 `fnmatch_internal(pat, -1, str, -1, flags)` 匹配整个字符串
- `m = -1` 和 `n = -1` 表示长度待由 `strnlen` 计算

### 不变量 (Invariants)

- `fnmatch` 不修改 `pat` 和 `str` 指向的内容（只读操作）
- `fnmatch` 无全局状态、无线程局部状态依赖（纯函数语义，除多字节转换可能需要 locale 数据外）

### 复杂度

- 最坏情况：`O(N * M)`，其中 `N = strlen(str)`，`M = strlen(pat)`
- 无 `*` 的简单模式：`O(min(N, M))`，因为阶段 1 头部匹配在首字节失配时即返回
- 多 `*` 模式：阶段 4 星海匹配对每个组件进行贪心搜索

---

## 引用关系

| 符号 | 类型 | 可见性 | 被引用者 |
|------|------|--------|----------|
| `fnmatch` | 函数 | Public | `<fnmatch.h>` |
| `fnmatch_internal` | 函数 | Internal | 仅 `fnmatch` 调用 |
| `str_next` | 函数 | Internal | `fnmatch_internal` 调用 |
| `pat_next` | 函数 | Internal | `fnmatch`、`fnmatch_internal` 调用 |
| `casefold` | 函数 | Internal | `fnmatch_internal` 调用 |
| `match_bracket` | 函数 | Internal | `fnmatch_internal` 调用 |
| `END` | 宏 | Internal | `pat_next`、`fnmatch_internal` 使用 |
| `UNMATCHABLE` | 宏 | Internal | `pat_next` 返回，`fnmatch_internal` 检测 |
| `BRACKET` | 宏 | Internal | `pat_next` 返回，`fnmatch_internal` 检测 |
| `QUESTION` | 宏 | Internal | `pat_next` 返回，`fnmatch_internal` 检测 |
| `STAR` | 宏 | Internal | `pat_next` 返回，`fnmatch_internal` 检测 |
