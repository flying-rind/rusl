# tre 内部类型定义 Rust 接口规约

## 概述

TRE 正则表达式引擎内部类型和常量定义模块。C 的 `tre.h` 头文件定义了 TNFA（Tagged Nondeterministic Finite Automaton）的全部数据结构、内存分配器抽象、宽字符处理包装宏，以及所有内部类型和常量。在 Rust 实现中，本模块对应 `rusl` crate 内部的 `tre` 子模块，所有符号均为 Internal 可见性（`pub(crate)` 或更小），不对外导出。

---

## 依赖图

```
tre 模块 (本模块)
  ├── <regex.h>     — POSIX 正则表达式公共 API（regex_t, regmatch_t 等）
  ├── <wchar.h>     — 宽字符类型
  └── <wctype.h>    — 宽字符分类/转换函数

tre 模块被以下子模块引用:
  ├── regcomp 模块  — 正则表达式编译
  ├── regexec 模块  — 正则表达式匹配
  └── tre_mem 模块  — 内存分配器
```

---

## [RELY]

Predefined Structures/Functions:
  `regex_t` / `regmatch_t` / `regoff_t` (type, `<regex.h>`)     // 依赖1: POSIX 公共类型
  `mbtowc` (fn, libc)                                             // 依赖2: 多字节到宽字符转换
  `iswalnum` / `iswalpha` / ... / `iswxdigit` (fn, libc)         // 依赖3: 宽字符分类函数
  `towlower` / `towupper` (fn, libc)                              // 依赖4: 宽字符大小写转换
  `iswctype` / `wctype` (fn, libc)                                // 依赖5: 泛型字符类别函数
  `wcslen` (fn, libc)                                             // 依赖6: 宽字符串长度
  `malloc` / `calloc` / `free` / `realloc` (fn, libc)             // 依赖7: 系统内存分配
  `std::ffi::c_int` / `std::ffi::c_void` 等 (Rust std)            // 依赖8: C ABI 类型映射

---

## [GUARANTEE]

本模块所有符号均为 Internal — 不对外导出。仅 rusl crate 内部使用。无对外导出接口。

---

## 第一部分：常量定义

### TRE_CHAR_MAX

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) const TRE_CHAR_MAX: i32 = 0x10ffff; // Unicode 最大合法码点 U+10FFFF
```

**意图**：定义 TNFA 中字符值的最大有效范围。任何 `c_int` 值若大于 `TRE_CHAR_MAX`，则被视为特殊标记（EMPTY、ASSERTION、TAG、BACKREF），而非实际字符。

### TRE_MEM_BLOCK_SIZE

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) const TRE_MEM_BLOCK_SIZE: usize = 1024;
```

**意图**：TRE 内存分配器每次从系统申请的内存块默认大小（1KB）。

### 断言位掩码常量

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) const ASSERT_AT_BOL: i32         = 1;    // 行首
pub(crate) const ASSERT_AT_EOL: i32         = 2;    // 行尾
pub(crate) const ASSERT_CHAR_CLASS: i32     = 4;    // 字符类别匹配（正向）
pub(crate) const ASSERT_CHAR_CLASS_NEG: i32 = 8;    // 字符类别匹配（反向）
pub(crate) const ASSERT_AT_BOW: i32         = 16;   // 词首
pub(crate) const ASSERT_AT_EOW: i32         = 32;   // 词尾
pub(crate) const ASSERT_AT_WB: i32          = 64;   // 词边界
pub(crate) const ASSERT_AT_WB_NEG: i32      = 128;  // 非词边界
pub(crate) const ASSERT_BACKREF: i32        = 256;  // 反向引用
pub(crate) const ASSERT_LAST: i32           = 256;  // 最后一个断言编号
```

**意图**：定义 TNFA 转移中 `assertions` 字段的位掩码值。断言是零宽度的匹配条件——不消耗输入字符，仅在转移时检查当前位置是否满足特定上下文条件。

---

## 第二部分：内部类型别名

### 宽字符相关类型

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) type TreCint = wint_t;    // 对应 C 的 tre_cint_t: 字符范围值类型（可容纳 WEOF）
pub(crate) type TreCtype = wctype_t; // 对应 C 的 tre_ctype_t: 宽字符类别句柄
```

**意图**：C 实现中 `tre_cint_t` / `tre_ctype_t` 为 `wint_t` / `wctype_t` 的 typedef 别名。Rust 中可直接使用对应 FFI 类型，或定义语义别名增强可读性。

### 宽字符包装函数（内联）

C 实现使用宏将 TRE 命名空间映射到 libc 宽字符函数（如 `#define tre_isalnum iswalnum`）。Rust 中可直接调用对应的 FFI 绑定，或封装为内联函数：

```rust
// [Visibility]: Internal — rusl crate 内部
// 以下函数直接映射到对应的 POSIX 宽字符分类函数
pub(crate) unsafe fn tre_isalnum(c: TreCint) -> bool { iswalnum(c) != 0 }
pub(crate) unsafe fn tre_isalpha(c: TreCint) -> bool { iswalpha(c) != 0 }
// ... 其余 10 个分类函数同理
pub(crate) unsafe fn tre_isspace(c: TreCint) -> bool { iswspace(c) != 0 }
pub(crate) unsafe fn tre_isxdigit(c: TreCint) -> bool { iswxdigit(c) != 0 }

// 大小写转换
pub(crate) unsafe fn tre_tolower(c: TreCint) -> TreCint { towlower(c) }
pub(crate) unsafe fn tre_toupper(c: TreCint) -> TreCint { towupper(c) }

// 泛型字符类别
pub(crate) unsafe fn tre_isctype(wc: TreCint, desc: TreCtype) -> bool { iswctype(wc, desc) != 0 }
pub(crate) unsafe fn tre_ctype(name: *const c_char) -> TreCtype { wctype(name) }
```

---

## 第三部分：TNFA 核心数据结构

### TagDirection — Tag 匹配方向枚举

```rust
// [Visibility]: Internal — rusl crate 内部
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagDirection {
    Minimize = 0,  // 对应 C 的 TRE_TAG_MINIMIZE: 非贪婪/懒惰匹配
    Maximize = 1,  // 对应 C 的 TRE_TAG_MAXIMIZE: 贪婪匹配（POSIX 最左最长规则）
}
```

**意图**：标记每个 submatch tag 的匹配策略——最小化匹配（`*?`、`+?`）或最大化匹配（`*`、`+`）。

### TnfaTransition — TNFA 转移边

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct TnfaTransition {
    pub code_min: TreCint,                // 接受的字符范围下限
    pub code_max: TreCint,                // 接受的字符范围上限（闭区间）
    pub state_id: i32,                    // 目标状态的数字 ID
    pub assertions: i32,                  // 断言位掩码
    pub tags: Option<Box<[i32]>>,         // 以 -1 结尾的 tag 编号列表（None = 无 tag）
    pub u_class: Option<TreCtype>,        // 字符类别（ASSERT_CHAR_CLASS 时）
    pub u_backref: Option<i32>,           // 反向引用编号（ASSERT_BACKREF 时）
    pub neg_classes: Option<Box<[TreCtype]>>, // 否定字符类别列表（以 0 结尾）
}
```

**C vs Rust 差异**：
- C 结构体使用 union `{ class, backref }`；Rust 使用 `Option` 枚举分别存储，语义更清晰
- C 的 `state` 指针指向目标状态的转移数组首地址；Rust 实现中将 TNFA 所有转移存储在扁平数组中，`state_id` 索引替代裸指针
- C 的 `tags` 以 `-1` 终止的裸指针数组；Rust 使用 `Option<Box<[i32]>>` 保证内存安全

**不变量**：
- 转移数组必须总是以 `state_id == END_STATE_MARKER` 的元素结尾
- `code_min <= code_max`（对于普通字符转移）
- `tags` 若非 None 则必须以 `-1` 结尾

### SubmatchData — 子匹配元数据

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct SubmatchData {
    pub so_tag: i32,                    // 提供 rm_so 值的 tag 编号
    pub eo_tag: i32,                    // 提供 rm_eo 值的 tag 编号
    pub parents: Option<Box<[i32]>>,    // 父 submatch 编号列表（以 0 结尾）
}
```

**意图**：为每个子表达式（捕获组）描述如何从 tag 值计算出 `regmatch_t` 中的 `rm_so`（起始偏移）和 `rm_eo`（结束偏移）。

### Tnfa — TNFA 顶层结构

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) struct Tnfa {
    pub transitions: Box<[TnfaTransition]>,  // 所有转移边的扁平数组
    pub initial_id: i32,                     // 初始状态的 state_id
    pub final_id: i32,                       // 接受（终止）状态的 state_id
    pub initial_tags: Option<Box<[i32]>>,    // 初始转移的 tag 列表
    pub submatch_data: Box<[SubmatchData]>,  // submatch 元数据数组
    pub firstpos_chars: [u8; 32],            // 位图（256-bit），可能匹配的首字符集合
    pub first_char: i32,                     // 确定的单个首字符（负值表示无）
    pub num_submatches: u32,                 // 子表达式（捕获组）总数
    pub tag_directions: Box<[TagDirection]>, // 每个 tag 的匹配方向
    pub minimal_tags: Option<Box<[i32]>>,    // 最小化匹配的 tag 编号列表（-1 结尾）
    pub num_tags: i32,                       // tag 总数
    pub num_minimals: i32,                   // 最小化匹配的 tag 数量
    pub end_tag: i32,                        // 整体匹配结束的 tag 编号
    pub num_states: i32,                     // TNFA 状态总数
    pub cflags: i32,                         // 编译标志
    pub have_backrefs: bool,                 // 是否包含反向引用
    pub have_approx: bool,                   // 是否使用近似匹配
}
```

**C vs Rust 关键差异**：

| C 字段 | C 类型 | Rust 字段 | Rust 类型 | 差异说明 |
|--------|--------|-----------|-----------|----------|
| `transitions` | `tre_tnfa_transition_t *` | `transitions` | `Box<[TnfaTransition]>` | Box 切片自动管理内存和长度 |
| `num_transitions` | `unsigned int` | — | (从 `transitions.len()` 获取) | 无需独立字段 |
| `initial` | `tre_tnfa_transition_t *` | `initial_id` + `initial_tags` | `i32` + `Option<Box<[i32]>>` | 用 state_id 替代裸指针 |
| `final` | `tre_tnfa_transition_t *` | `final_id` | `i32` | 同理 |
| `submatch_data` | `tre_submatch_data_t *` | `submatch_data` | `Box<[SubmatchData]>` | Box 切片代替裸指针 |
| `tag_directions` | `tre_tag_direction_t *` | `tag_directions` | `Box<[TagDirection]>` | Box 切片 |
| `minimal_tags` | `int *` | `minimal_tags` | `Option<Box<[i32]>>` | Option 表示可为空 |
| `have_backrefs` | `int` | `have_backrefs` | `bool` | 语义更清晰 |

**不变量**：
- `num_submatches >= 1`（第 0 号始终存在，对应整体匹配）
- `submatch_data.len() == num_submatches as usize`
- `tag_directions.len() == num_tags as usize`
- `firstpos_chars` 长度为 32 字节
- 若 `have_backrefs` 为真，匹配引擎必须启用反向引用解析路径
- TNFA 对象由 `regcomp` 构造，由 `regfree` 释放（通过 Drop）

---

## 第四部分：系统分配器别名

C 实现使用 `xmalloc` / `xcalloc` / `xfree` / `xrealloc` 宏映射到 libc 分配函数。Rust 实现中直接使用 Rust 的 `Box`、`Vec`、`alloc::alloc` 等安全抽象，无需这些别名。若确实需要裸分配（如 ABI 兼容场景），使用 `std::alloc::Global` 或 FFI 绑定。

```rust
// [Visibility]: Internal — 仅供需要直接调用 libc 的场景使用
pub(crate) unsafe fn xmalloc(size: usize) -> *mut c_void { libc::malloc(size) }
pub(crate) unsafe fn xcalloc(n: usize, size: usize) -> *mut c_void { libc::calloc(n, size) }
pub(crate) unsafe fn xfree(ptr: *mut c_void) { libc::free(ptr) }
pub(crate) unsafe fn xrealloc(ptr: *mut c_void, size: usize) -> *mut c_void { libc::realloc(ptr, size) }
```

---

## 第五部分：工具函数

### elementsof — 数组元素个数

C 的 `#define elementsof(x) (sizeof(x)/sizeof(x[0]))` 在 Rust 中对应为切片 `.len()` 或数组 `.len()` 方法，无需独立宏/函数。

### ALIGN — 指针对齐计算

```rust
// [Visibility]: Internal — rusl crate 内部
pub(crate) const fn align_offset(ptr: usize, type_align: usize) -> usize {
    if ptr % type_align == 0 { 0 } else { type_align - ptr % type_align }
}
```

**意图**：计算将 `ptr` 对齐到指定对齐边界所需的字节偏移量。

---

## 符号导出状态汇总

所有符号均为 Internal (`pub(crate)` 或更小可见性) — 不对外导出：

| 符号 | 类别 | 说明 |
|------|------|------|
| `TRE_CHAR_MAX` | const | Unicode 最大码点 |
| `TRE_MEM_BLOCK_SIZE` | const | 分配器块大小 |
| `ASSERT_AT_BOL` ~ `ASSERT_LAST` | const | TNFA 断言位掩码 |
| `TreCint` / `TreCtype` | type alias | 宽字符类型别名 |
| `TagDirection` | enum | tag 匹配方向 |
| `TnfaTransition` | struct | TNFA 转移边 |
| `SubmatchData` | struct | submatch 元数据 |
| `Tnfa` | struct | TNFA 顶层结构 |
| `align_offset` | fn | 指针对齐计算工具 |
| `tre_is*` 系列 | fn | 宽字符分类包装 |

---

## 与其他模块的关系

| 相关模块 | 关系 |
|----------|------|
| `tre_mem` 模块 | 使用 `TRE_MEM_BLOCK_SIZE` 和 `Tnfa` 类型 |
| `regcomp` 模块 | 构造 `Tnfa` 作为编译输出；使用所有内部类型 |
| `regexec` 模块 | 遍历 `Tnfa` 执行匹配；解释断言位掩码和 tag 记录 |
| `<regex.h>` (POSIX) | 定义 `regex_t`（包含 opaque 指针指向 `Tnfa`） |
