# regexec Rust 接口规约

## 概述

本模块实现 POSIX `regexec()` 函数，对字符串执行已编译正则表达式的匹配。根据正则是否包含反向引用，分派到并行匹配器（`tre_tnfa_run_parallel`）或回溯匹配器（`tre_tnfa_run_backtrack`）。Rust 实现中，对外 `regexec` 签名保持 ABI 兼容，内部匹配引擎可用 Rust 安全抽象和所有权模型重构。

---

## 依赖图

```
regexec (Public)
  ├── tre_tnfa_run_parallel (Internal)     — 并行 NFA 模拟匹配器
  │     ├── tre_fill_pmatch (Internal)
  │     ├── tre_tag_order (Internal)
  │     ├── tre_neg_char_classes_match (Internal)
  │     └── GET_NEXT_WCHAR / CHECK_ASSERTIONS / CHECK_CHAR_CLASSES (宏/内联)
  └── tre_tnfa_run_backtrack (Internal)    — 深度优先回溯匹配器
        ├── tre_fill_pmatch (Internal)
        ├── tre_tag_order (Internal)
        ├── tre_neg_char_classes_match (Internal)
        ├── tre_mem 分配器 (Internal)
        └── BT_STACK_PUSH / BT_STACK_POP (宏/内联)

类型依赖:
  regexec ──> regex_t, regmatch_t (see <regex.h>)
  regexec ──> Tnfa, TnfaTransition, SubmatchData (see tre 模块)

外部模块依赖:
  tre_mem 模块 (Internal)           — 回溯栈内存管理
  mbtowc / iswalnum / iswctype 等 (libc) — 宽字符处理
```

---

## [RELY]

Predefined Structures/Functions:
  `regex_t` / `regmatch_t` / `regoff_t` (type, `<regex.h>`)        // 依赖1: POSIX 公共类型
  `Tnfa` / `TnfaTransition` / `SubmatchData` (struct, tre 模块)     // 依赖2: TNFA 核心数据结构
  `TagDirection` (enum, tre 模块)                                   // 依赖3: tag 匹配方向
  `TreCint` / `TreCtype` (type alias, tre 模块)                     // 依赖4: 宽字符类型
  `tre_mem_alloc` / `tre_mem_destroy` (fn, tre_mem 模块)            // 依赖5: 内存分配器
  `mbtowc` (fn, libc)                                               // 依赖6: 多字节到宽字符转换
  `iswalnum` / `iswctype` / `towlower` / `towupper` (fn, libc)     // 依赖7: 宽字符分类/转换
  `calloc` / `free` / `memset` / `strncmp` (fn, libc)              // 依赖8: 标准库函数
  `c_char`, `c_int`, `c_size_t` (std::ffi / libc 类型)              // 依赖9: C ABI 兼容类型
  `MB_LEN_MAX` (system constant)                                    // 依赖10: 多字节字符最大字节数

---

## [GUARANTEE]

Exported Interface:

```rust
extern "C" fn regexec(preg: *const regex_t, string: *const c_char, nmatch: size_t, pmatch: *mut regmatch_t, eflags: c_int) -> c_int
```

本模块保证对外提供的接口签名，ABI 兼容 POSIX `regexec()`。

---

## 内部类型定义

### ReachState — 并行匹配器的可达状态

C 实现中以 `tre_tnfa_reach_t` 结构体表示并行匹配器中某一可达路径的状态。Rust 实现中可重新设计：

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct ReachState {
    pub state_id: i32,                 // 当前所处 TNFA 状态的 ID
    pub tags: Box<[regoff_t]>,         // 标签值数组（记录各捕获组的起始偏移）
}
```

**意图**：表示并行匹配器（`tnfa_run_parallel`）中某一可达路径的状态。`tags` 数组长度等于 `tnfa.num_tags`。

### ReachPos — 每状态已访问记录

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct ReachPos {
    pub pos: regoff_t,                 // 该状态最近一次被访问时的字符位置
    pub tags: Option<Box<[regoff_t]>>, // 该状态对应的最佳标签值数组
}
```

**意图**：记录 TNFA 某个 `state_id` 最近一次被访问时的字符位置和最佳标签数组指针，用于多路径去重优化。

### BacktrackItem — 回溯栈帧

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct BacktrackItem {
    pub pos: regoff_t,             // 字符位置
    pub str_byte: usize,           // 字符串字节偏移（替代 C 的裸指针）
    pub state_id: i32,             // 当前 TNFA 状态 ID
    pub next_c: TreCint,           // 下一个宽字符预览
    pub tags: Box<[regoff_t]>,     // 标签值数组
}
```

**意图**：回溯匹配器栈中的一个帧，保存回溯点的完整上下文。Rust 实现中使用 `Box<[regoff_t]>` 替代 C 的裸指针 `tags`。

### BacktrackStack — 回溯栈

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct BacktrackStack {
    pub stack: Vec<BacktrackItem>,     // 栈帧数组（替代 C 的双向链表）
    pub sp: usize,                     // 栈指针
}
```

**Rust 设计优势**：
- C 实现使用双向链表 + `tre_mem` 分配器管理栈帧；Rust 直接使用 `Vec<BacktrackItem>`，由 Rust 标准库管理内存
- `Vec` 的 `push`/`pop` 方法与栈语义天然匹配
- 复用已分配容量（类似 C 的 `stack->next` 复用机制）由 `Vec` 自动实现

---

## 内部辅助函数

### tre_neg_char_classes_match — 否定字符类匹配

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_neg_char_classes_match(
    classes: &[TreCtype],   // 以 0 结尾的字符类列表
    wc: TreCint,             // 待检查的宽字符
    icase: bool,             // 是否大小写不敏感
) -> bool
```

**意图**：检查给定宽字符 `wc` 是否属于否定字符类列表 `classes` 中的任一字符类。

**前置条件**：
- `classes` 以 0 结尾
- `wc` 为有效的宽字符

**后置条件**：
- Case 1（匹配成功）：返回 `true` — `wc` 属于 `classes` 中至少一个字符类
- Case 2（无匹配）：返回 `false` — `wc` 不属于 `classes` 中的任何一个字符类
- 大小写不敏感时：`wc` 的大写或小写形式之一属于某字符类即算匹配

---

### tre_tag_order — 标签排序比较

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_tag_order(
    tag_directions: &[TagDirection],
    t1: &[regoff_t],
    t2: &[regoff_t],
) -> bool  // true 表示 t1 优于 t2
```

**意图**：比较两套标签值 `t1` 和 `t2`，按 TNFA 定义的 `tag_directions`（每个标签是最小化还是最大化）逐位词典序判断 `t1` 是否"优于"`t2`。

**前置条件**：
- `tag_directions.len() == t1.len() == t2.len() > 0`

**后置条件**：
- Case 1（t1 胜出）：返回 `true` — 逐标签比较，在第一个 `t1[i] != t2[i]` 的位置，若方向为 Minimize 且 `t1[i] < t2[i]`，或方向为 Maximize 且 `t1[i] > t2[i]`
- Case 2（t1 不胜出）：返回 `false` — `t2` 在所有分歧位上均不劣于 `t1`

---

### tre_fill_pmatch — 填充 regmatch_t 数组

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tre_fill_pmatch(
    nmatch: usize,
    pmatch: &mut [regmatch_t],
    cflags: c_int,
    tnfa: &Tnfa,
    tags: &[regoff_t],
    match_eo: regoff_t,
)
```

**意图**：在匹配成功后，根据编译期收集的子匹配数据和运行期收集的标签终点偏移，按左最长匹配的 POSIX 语义填充 `regmatch_t` 数组。

**前置条件**：
- `pmatch.len() >= nmatch`
- `tags.len() >= tnfa.num_tags as usize`
- `tnfa.submatch_data` 有效

**后置条件**：
- 对 `i < min(nmatch, tnfa.num_submatches)`：`pmatch[i].rm_so` / `pmatch[i].rm_eo` 根据对应 tag 值填充
- 若 `cflags & REG_NOSUB`：所有 `pmatch[i] = {-1, -1}`
- 子匹配父子约束修正：不满足父子包含关系的子匹配重置为 `{-1, -1}`
- 不变量：`pmatch[i].rm_so == -1` 蕴含 `pmatch[i].rm_eo == -1`

---

## 匹配引擎

### tnfa_run_parallel — 并行 NFA 模拟匹配器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tnfa_run_parallel(
    tnfa: &Tnfa,
    string: &[u8],                    // 待匹配的多字节字符串
    match_tags: Option<&mut [regoff_t]>, // 标签值输出缓冲区
    eflags: c_int,
    match_end_ofs: &mut regoff_t,
) -> RegError
```

**意图**：实现 POSIX 左最长匹配的并行 NFA 模拟算法。所有匹配路径同时推进，到达同一状态时按标签方向规则择一保留。该算法**不能**处理包含反向引用的正则表达式（此时应使用 `tnfa_run_backtrack`）。

**系统算法**：
1. **初始化**：分配 `reach` / `reach_next` 两个 Vec（当前轮/下一轮可达状态）、`reach_pos` 数组（每个状态最近访问记录）
2. **初始状态加入**：扫描初始状态的转换，检查断言，通过的加入 `reach_next`
3. **主循环**（逐字符推进）：
   - 读取下一宽字符（`mbtowc`）
   - 交换 `reach` 和 `reach_next`
   - **最小匹配剔除**：丢弃不满足最小匹配条件的状态
   - **转换探索**：对每个可达状态尝试其所有出边，检查字符范围、断言和字符类
   - 若到达终态且本次路径不差于已知最优，更新 `match_eo` 和 `match_tags`
4. **终止**：当字符串读完且 `reach_next` 为空时跳出
5. **清理**：释放临时 Vec

**前置条件**：
- `tnfa` 指向已编译的 TNFA
- `string` 为有效的字节切片（以 `\0` 结尾）
- `match_tags` 若为 `Some`，长度至少 `tnfa.num_tags`

**后置条件**：

| 条件 | 返回值 | `*match_end_ofs` |
|------|--------|------------------|
| 匹配成功 | `RegError::Ok` | 左最长匹配的结束偏移 (>= 0) |
| 无匹配 | `RegError::NoMatch` | -1 |
| 内存不足 | `RegError::Error(REG_ESPACE)` | 未定义 |

**时间复杂度**：`O(|string| * |states| * |transitions_per_state|)`

---

### tnfa_run_backtrack — 深度优先回溯匹配器

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) fn tnfa_run_backtrack(
    tnfa: &Tnfa,
    string: &[u8],
    match_tags: Option<&mut [regoff_t]>,
    eflags: c_int,
    match_end_ofs: &mut regoff_t,
) -> RegError
```

**意图**：实现带反向引用支持的正则表达式匹配。使用深度优先回溯搜索在 TNFA 中探索所有可能路径，确保返回左最长匹配。带反向引用的正则匹配是 NP 完全的，回溯是最通用的算法（可能极慢甚至耗尽栈空间）。

**系统算法**：
1. **初始化**：分配回溯栈（`Vec<BacktrackItem>`）、标签数组、`states_seen` 数组（防止无限的零长度反向引用循环）
2. **起始位置尝试**：从字符串每个位置开始扫描
3. **初始状态处理**：扫描初始转换，通过断言检查的进入探索；其余通过栈保存为备选路径
4. **主循环**：
   - **到达终态**：比较当前匹配是否优于已知最优，若是则更新。然后无条件回溯。
   - **反向引用处理**：调用 `tre_fill_pmatch` 获取被引用子匹配的实际区间，用 `strncmp` 比较。匹配成功则跳过对应长度继续；失败则回溯。
   - **普通字符匹配**：读取下一字符，在出边中查找字符范围匹配的转换。第一个直接进入，其余压栈备选。
   - **转换失败**：回溯
5. **回溯逻辑**：从栈顶恢复上下文；若栈空且 `match_eo < 0`，将起始位置后移
6. **终止**：当栈空且 `match_eo >= 0`（找到匹配）或所有起始位置均尝试完毕

**前置条件**：同 `tnfa_run_parallel`，但 TNFA 可能包含反向引用。

**后置条件**：同 `tnfa_run_parallel`。

**时间复杂度**：最坏情况为指数级（NP 完全问题本质），实际使用中很少触发。

---

## regexec (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn regexec(
    preg: *const regex_t,
    string: *const c_char,
    nmatch: size_t,
    pmatch: *mut regmatch_t,
    eflags: c_int,
) -> c_int
```

[Visibility]: Public — POSIX 标准函数，`<regex.h>` 声明。

### 意图 (Intent)

对 `string` 执行 `preg` 对应的已编译正则表达式的匹配。根据正则是否包含反向引用，分派到并行匹配器或回溯匹配器。

### 前置条件

- `preg` 指向通过 `regcomp()` 成功编译的 `regex_t` 对象，且未被 `regfree()` 释放
- `string` 指向以 NUL 结尾的多字节字符串
- `nmatch` 为 `pmatch` 数组的元素个数（可为 0）
- 若 `nmatch > 0`，`pmatch` 指向长度至少 `nmatch` 的有效数组
- `eflags` 为 0 或 `REG_NOTBOL | REG_NOTEOL` 的组合

### 后置条件

| 条件 | 返回值 | `pmatch` 状态 |
|------|--------|---------------|
| 匹配成功 | `REG_OK` (0) | `pmatch[0]` 为整体匹配区间；`pmatch[1..]` 为子组捕获区间 |
| 无匹配 | `REG_NOMATCH` (1) | 内容未定义 |
| 内存不足 | `REG_ESPACE` (12) | 内容未定义 |

### 系统算法（分派逻辑）

```
1. 从 preg.__opaque 提取 &Tnfa
2. 若编译时指定 REG_NOSUB: 强制 nmatch = 0
3. 若需要捕获组信息 (num_tags > 0 && nmatch > 0): 分配标签数组
4. 分派匹配引擎:
   - tnfa.have_backrefs == true  → tnfa_run_backtrack
   - 否则                        → tnfa_run_parallel
5. 若匹配成功 (REG_OK): 调用 tre_fill_pmatch 填充 pmatch
6. 释放标签数组，返回状态码
```

### 不变量 (Invariants)

- `regexec` 不修改 `preg` 指向的 TNFA 结构（只读操作）
- 不修改 `string` 指向的内容（只读操作）
- 线程安全：所有临时数据在栈/Rust Vec 上分配，不修改全局状态

### POSIX 符合性

完全实现 POSIX.1-2001 的 `regexec()` 语义，包括：
- 左最长匹配规则
- `REG_NOTBOL` / `REG_NOTEOL` 标志
- 子匹配父子约束修正
- 未参与匹配的子组返回 `{-1, -1}`

---

## 引用关系

| 符号 | 可见性 | 被引用者 |
|------|--------|----------|
| `regexec` | Public | `<regex.h>` |
| `tnfa_run_parallel` | Internal (`pub(crate)`) | `regexec` 调用 |
| `tnfa_run_backtrack` | Internal (`pub(crate)`) | `regexec` 调用 |
| `tre_fill_pmatch` | Internal (`pub(crate)`) | 两个匹配引擎调用 |
| `tre_tag_order` | Internal (`pub(crate)`) | 两个匹配引擎调用 |
| `tre_neg_char_classes_match` | Internal (`pub(crate)`) | CHECK_CHAR_CLASSES 调用 |
| `ReachState` | Internal (`pub(crate)`) | `tnfa_run_parallel` 使用 |
| `ReachPos` | Internal (`pub(crate)`) | `tnfa_run_parallel` 使用 |
| `BacktrackItem` | Internal (`pub(crate)`) | `tnfa_run_backtrack` 使用 |
| `BacktrackStack` | Internal (`pub(crate)`) | `tnfa_run_backtrack` 使用 |
