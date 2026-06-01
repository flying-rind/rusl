# fnmatch Rust 接口规约

## 概述

本模块实现 POSIX `fnmatch()` 函数，采用 Rich Felker 设计的 "Sea of Stars" 算法。Rust 实现中，外部 `fnmatch` 签名保持 ABI 兼容，内部辅助函数（`pat_next`、`str_next`、`casefold`、`match_bracket`、`fnmatch_internal`）可使用 Rust 安全抽象重构，内部宏常量（`END`、`STAR` 等）可用 Rust 枚举替代。

---

## 依赖图

```
fnmatch (Public)
  └── fnmatch_internal (Internal) — 核心匹配引擎
        ├── pat_next (Internal)    — 模式 token 读取器
        ├── str_next (Internal)    — 字符串字符读取器
        ├── casefold (Internal)    — 大小写折叠
        └── match_bracket (Internal) — 方括号表达式匹配

外部依赖（不在此生成规约）:
  - mbtowc (<wchar.h>)           — 多字节到宽字符转换
  - towupper, towlower, iswctype, wctype (<wctype.h>) — 宽字符分类/转换
  - FNM_* 标志宏 (<fnmatch.h>)
```

---

## [RELY]

Predefined Structures/Functions:
  `mbtowc` (fn, libc)                                    // 依赖1: 多字节到宽字符转换
  `towupper` / `towlower` (fn, libc)                      // 依赖2: 宽字符大小写转换
  `iswctype` / `wctype` (fn, libc)                        // 依赖3: 宽字符类别匹配
  `c_char`, `c_int`, `c_size_t` (std::ffi / libc 类型)    // 依赖4: C ABI 兼容类型

---

## [GUARANTEE]

Exported Interface:

```rust
extern "C" fn fnmatch(pat: *const c_char, str: *const c_char, flags: c_int) -> c_int
```

本模块保证对外提供的接口签名，ABI 兼容 POSIX `fnmatch()`。

---

## 内部类型定义

### TokenKind — 模式 Token 枚举

C 实现使用负值宏（`END = 0`、`STAR = -5` 等）编码特殊 token 类型。Rust 实现用枚举替代，更类型安全且语义清晰。

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) enum TokenKind {
    End,            // 对应 C 的 END (0): 模式/字符串结束
    Unmatchable,    // 对应 C 的 UNMATCHABLE (-2): 非法多字节序列
    Bracket,        // 对应 C 的 BRACKET (-3): 方括号表达式 [...]
    Question,       // 对应 C 的 QUESTION (-4): 问号 ? 通配符
    Star,           // 对应 C 的 STAR (-5): 星号 * 通配符
    Literal(i32),   // 对应 C 的非负返回值: 字面字符（Unicode 码点）
}
```

---

## 内部辅助函数

### str_next — 字符串字符读取器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn str_next(str: &[u8], pos: &mut usize) -> TokenKind
```

**意图**：从字符串字节切片中读取下一个字符（可能是多字节 UTF-8 字符），推进位置指针。这是 "Sea of Stars" 算法中所有字符串遍历的字符级原子操作。

**前置条件**：
- `str` 为有效的字节切片
- `pos` 为切片中的有效位置

**后置条件**：

| 条件 | 返回值 | `*pos` 变化 |
|------|--------|------------|
| `pos >= str.len()`（无可用字符） | `TokenKind::End` | 不变 |
| `str[*pos] < 128`（单字节 ASCII） | `TokenKind::Literal(str[*pos] as i32)` | `*pos += 1` |
| `str[*pos] >= 128`（多字节 UTF-8） | 由 `mbtowc` 决定 | 见下方 |
| `mbtowc` 成功解码 k 字节 | `TokenKind::Literal(wc)` | `*pos += k` |
| `mbtowc` 解码失败（非法序列） | `TokenKind::Unmatchable` | `*pos += 1` |

**不变量**：
- `*pos` 推进量永不超过剩余字符串长度
- 返回 `TokenKind::Unmatchable` 时表示需要跳过 1 字节非法序列

---

### pat_next — 模式 Token 读取器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn pat_next(pat: &[u8], pos: &mut usize, flags: FnmFlags) -> TokenKind
```

**意图**：从模式字节切片中读取下一个 token。Token 可以是特殊通配符（`*`、`?`、`[...]`）、字面字符、或为非法序列标记。

`FnmFlags` 为 `bitflags` 类型，包含 `PATHNAME`、`NOESCAPE`、`PERIOD`、`LEADING_DIR`、`CASEFOLD` 位。

**前置条件**：
- `pat` 为有效的字节切片
- `pos` 为切片中的有效位置
- `flags` 包含有效的标志位组合

**后置条件**：

| 条件 | 返回值 | `*pos` 变化 |
|------|--------|------------|
| `pos >= pat.len()` 或 `pat[*pos] == b'\0'` | `TokenKind::End` | `*pos += 0` |
| `pat[*pos] == b'\\'` 且转义未禁用且后续字符非空 | 处理转义 | 跳过反斜杠+处理后续字符 |
| `pat[*pos] == b'['` | `TokenKind::Bracket` | `*pos += bracket_len` |
| `pat[*pos] == b'*'` | `TokenKind::Star` | `*pos += 1` |
| `pat[*pos] == b'?'` | `TokenKind::Question` | `*pos += 1` |
| 多字节字面字符 | `TokenKind::Literal(wc)` | `*pos += k` |
| 单字节字面字符 | `TokenKind::Literal(pat[*pos] as i32)` | `*pos += 1` |

**不变量**：
- `*pos` 推进量永不超过剩余模式长度
- 返回 `TokenKind::Unmatchable` 表示模式中存在非法多字节序列，此模式整体不可能匹配成功

---

### casefold — 大小写折叠

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn casefold(k: i32) -> i32
```

**意图**：对单个字符进行大小写折叠。策略：先尝试 `towupper(k)`，若结果等于 `k`（即原字符已是大写或无大写形式），则尝试 `towlower(k)`。

**前置条件**：`k` 为有效的字符码点。

**后置条件**：
- 若 `towupper(k) != k`：返回 `towupper(k)`
- 否则返回 `towlower(k)`

---

### match_bracket — 方括号表达式匹配

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn match_bracket(
    p: &[u8],           // 指向 [ 之后的字符切片
    k: i32,             // 待匹配的字符码点
    kfold: i32,         // k 的大小写折叠形式
    eflags: i32,        // 执行标志（REG_NEWLINE 等）
) -> bool
```

**意图**：判断字符 `k`（或其大小写折叠形式 `kfold`）是否匹配方括号表达式 `[...]`。支持取反 `[^...]` / `[!...]`、字符范围 `a-z`、字符类 `[:class:]`。

**前置条件**：
- `p` 指向 `[` 之后的字节切片
- `k` 和 `kfold` 为有效字符码点（若大小写不匹配模式下 `kfold == k`）

**后置条件**：
- Case 1（匹配成功）：返回 `true`
- Case 2（匹配失败）：返回 `false`
- Case 3（模式非法，如解码失败）：返回 `false`

**不变量**：
- 函数不会读取越过首个未转义的 `]` 之后
- 扫描过程中若遇到多字节解码失败，返回 `false`

---

## fnmatch_internal — 核心匹配引擎

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn fnmatch_internal(
    pat: &[u8],
    str: &[u8],
    flags: FnmFlags,
) -> bool
```

**意图**：使用 "Sea of Stars" 算法进行模式匹配，返回 `true` 表示匹配成功，`false` 表示匹配失败（对应 C 的 `FNM_NOMATCH`）。

### 系统算法 (System Algorithm)

"Sea of Stars" 算法 — 将模式分解为头部、尾部和由 `*` 分隔的中间组件：

1. **`FNM_PERIOD` 前缀检查**：若 `flags & FNM_PERIOD`，检查 `str` 首字符为 `.` 时模式是否以 `.` 显式匹配。

2. **头部匹配 (Head Match)**：从模式开头匹配到第一个 `*`，若失配则立即返回 `false`。此阶段无需知道字符串长度。

3. **尾部收集 (Tail Collection)**：重新扫描整个模式，定位最后一个 `*` 及其后的所有字面 token，累计 `tailcnt`。

4. **尾部匹配 (Tail Match)**：从字符串末尾提取 `tailcnt` 个字符，与模式尾部逐 token 比较。

5. **星海匹配 (Sea of Stars)**：在头部和尾部之间，找到由 `*` 分隔的每个组件，按顺序在字符串中搜索其首次出现处。若某组件找不到匹配位置，则推进起始位置并重试。

**前置条件**：
- `pat` 和 `str` 为有效的字节切片（可能以 `\0` 结尾或精确截取的长度）
- `flags` 包含 `FNM_*` 标志位组合

**后置条件**：
- Case 1（匹配成功）：返回 `true`（对应 C 的 `0`）
- Case 2（匹配失败）：返回 `false`（对应 C 的 `FNM_NOMATCH`）

**不变量**：
- 字符串遍历不越界
- 尾部匹配阶段中边界不会被修改

---

## fnmatch (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn fnmatch(
    pat: *const c_char,
    str: *const c_char,
    flags: c_int,
) -> c_int
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<fnmatch.h>` 声明。

### 意图 (Intent)

测试字符串 `str` 是否与 shell 通配符模式 `pat` 匹配。

### 前置条件

- `pat` 和 `str` 必须是以空字符结尾的有效 C 字符串
- `flags` 是 `FNM_*` 标志的位或组合：
  - `FNM_PATHNAME` (0x01): 路径名模式 — `*` 和 `?` 不匹配 `/`
  - `FNM_NOESCAPE` (0x02): 禁用 `\` 转义
  - `FNM_PERIOD` (0x04): 前导句点必须被显式匹配
  - `FNM_LEADING_DIR` (0x08): 前导目录匹配视为整体匹配成功
  - `FNM_CASEFOLD` (0x10): 大小写折叠匹配

### 后置条件

| 条件 | 返回值 |
|------|--------|
| 匹配成功 | `0` |
| 匹配失败 | `FNM_NOMATCH` (1) |
| 系统不支持 | `FNM_NOSYS` (-1) — musl 实现从不返回此值 |

### 行为详细描述

#### `FNM_PATHNAME` 模式

采用逐路径段匹配策略：
1. 在 `str` 和 `pat` 中逐段寻找 `/` 分隔符
2. 对每一段调用 `fnmatch_internal` 进行段内匹配
3. 若某段匹配失败：返回 `FNM_NOMATCH`
4. 所有段匹配完成：返回 `0`

#### `FNM_LEADING_DIR` 模式（无 `FNM_PATHNAME`）

- 扫描字符串中每个 `/` 位置
- 对字符串前缀调用 `fnmatch_internal` 尝试匹配
- 若任意前缀匹配成功：返回 `0`
- 否则最终尝试整个字符串匹配

#### 普通模式（无 `FNM_PATHNAME` 和 `FNM_LEADING_DIR`）

- 直接调用 `fnmatch_internal(pat, str, flags)` 匹配整个字符串

### 不变量 (Invariants)

- `fnmatch` 不修改 `pat` 和 `str` 指向的内容（只读操作）
- `fnmatch` 无全局状态、无线程局部状态依赖（纯函数语义）

### 复杂度

- 最坏情况：`O(N * M)`，其中 `N = strlen(str)`，`M = strlen(pat)`
- 无 `*` 的简单模式：`O(min(N, M))`
- 多 `*` 模式：阶段 4 星海匹配对每个组件进行贪心搜索

---

## 引用关系

| 符号 | 可见性 | 被引用者 |
|------|--------|----------|
| `fnmatch` | Public | `<fnmatch.h>` |
| `fnmatch_internal` | Internal (`pub(crate)`) | 仅 `fnmatch` 调用 |
| `str_next` | Internal (`pub(crate)`) | `fnmatch_internal` 调用 |
| `pat_next` | Internal (`pub(crate)`) | `fnmatch`、`fnmatch_internal` 调用 |
| `casefold` | Internal (`pub(crate)`) | `fnmatch_internal` 调用 |
| `match_bracket` | Internal (`pub(crate)`) | `fnmatch_internal` 调用 |
| `TokenKind` | Internal (`pub(crate)`) | 各内部函数使用 |
| `FnmFlags` | Internal (`pub(crate)`) | 各内部函数使用 |
