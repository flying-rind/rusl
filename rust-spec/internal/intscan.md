# intscan 规约 (Rust)

## 概述

`intscan` 模块提供 rusl 内部的整数流解析功能。它是 `strtol`/`strtoul`/`strtoll`/`strtoull` 以及 `scanf` 系列 `%d`/`%i`/`%u`/`%x`/`%o` 转换的底层解析引擎。

## 依赖图

```
intscan 模块
├── 内部 FILE 结构体 (提供流 I/O 原语: shgetc, shunget, shlim)
├── 256 字节字符分类查找表 (静态常量, 编译期生成)
└── 调用者:
    ├── strtol / strtoul / strtoll / strtoull (字符串 → 整数)
    └── vfscanf (格式化输入)
```

---

```
/* Rely */
[RELY]
内部依赖:
  struct FILE { ... };                       // 依赖1: 内部文件流结构体
  fn shgetc(f: *mut FILE) -> c_int;          // 依赖2: 流中读取一个字符（信号安全）
  fn shunget(f: *mut FILE);                  // 依赖3: 回退一个字符到流中
  fn shlim(f: *mut FILE, lim: c_int);        // 依赖4: 设置流读取宽度限制

[GUARANTEE]
内部接口:
  fn __intscan(f: *mut FILE, base: c_uint, pok: c_int, lim: c_ulonglong) -> c_ulonglong;
                                  // 本模块保证提供的接口签名
                                  // 函数行为满足下方所有前置/后置条件和不变量
```

---

## 符号规约

### __intscan

```rust
// Rust — 内部整数流解析引擎
fn __intscan(
    f: *mut FILE,       // 内部 FILE 流指针, 调用前需由 shlim(f, width) 设置读取宽度限制
    base: c_uint,       // 进制基数: 0(自动检测), 2..=36 合法; base==1 或 base>36 非法
    pok: c_int,         // peek-only: 0=消费模式(正常解析), 非0=仅窥探模式(失败时可回溯全部已读字符)
    lim: c_ulonglong,   // 溢出上限。bit 0 语义: 1=无符号溢出方向, 0=有符号溢出方向
) -> c_ulonglong;       // 返回解析出的整数值(c_ulonglong), 调用者可按需 reinterpret 为有符号类型
```

[Visibility]: Internal (不导出) — rusl 内部整数解析引擎。被 `strtol`/`strtoul`/`strtoll`/`strtoull` (`src/stdlib/strtol.rs`) 和 `vfscanf` (`src/stdio/vfscanf.rs`) 调用。POSIX/C 标准未定义此符号。

**Intent**: rusl 将所有文本到整数的转换统一到一个函数中。`strtol`/`strtoul` 通过创建内存 FILE 流并调用 `__intscan` 实现；`scanf` 的 `%d`/`%i`/`%u`/`%x`/`%o` 转换也委托给 `__intscan`。这种设计避免了每个整数类型和每种输入源组合都独立实现的代码膨胀。

---

#### 参数契约

| 参数 | 契约 |
|------|------|
| `f` | 指向已初始化的 FILE 流。调用前应由 `shlim(f, width)` 设置读取宽度限制。不可为 null |
| `base` | 进制基数。合法值: `0`(自动检测)、`[2, 36]`。`base == 1` 或 `base > 36` 非法 |
| `pok` | "peek only" 标志。`0` = 消费模式(正常解析，消耗字符)，非零 = 仅窥探模式(失败时可回溯全部已读字符，流位置不变) |
| `lim` | 溢出上限。最低位 `(lim & 1)` 有特殊语义: `1` = 无符号溢出方向(允许 negative overflow 语义), `0` = 有符号溢出方向 |

---

#### 前置条件 (Pre-conditions)

1. `f != null` — 流指针有效且内部缓冲区已初始化
2. `f` 已由调用者通过 `shlim(f, width)` 设置读取宽度限制(来自 `scanf` 的 field width 或 `strtol` 的字符串长度)
3. `base` in `{0} ∪ [2, 36]`
4. `lim` 设置为对应目标类型的溢出上限值:
   - `unsigned int`: `lim = UINT_MAX`, `(lim & 1) == 1` (无符号语义)
   - `signed long long`: `lim = (unsigned long long)LLONG_MAX + 1`, `(lim & 1) == 0` (有符号语义)

---

#### 后置条件 (Post-conditions) — 按返回值案例分析

**Case A — 成功解析**:
- 返回值 = 解析出的整数值(`c_ulonglong` 类型，调用者可按需 reinterpret 为有符号类型)
- `errno` 不被修改(保留调用前的值)
- 流位置前进至第一个非数字字符

**Case B — 无有效数字字符**:
- 返回 `0`
- `errno = EINVAL`
- 流位置不变或已调用 `shlim(f, 0)` 终止读取

**Case C — 溢出**:
- 返回 `lim`(正溢出) 或 `lim-1`(负溢出/无符号溢出)
- `errno = ERANGE`
- 所有溢出数字已被消费，流位置前进至第一个非数字字符

**Case D — 非法基数** (`base > 36 || base == 1`):
- 返回 `0`
- `errno = EINVAL`

---

#### System Algorithm (Level 3 — 高度优化)

`__intscan` 采用**分层累加 + 查表 + 按基分派**策略：

1. **256 字节查找表**: 静态 `const TABLE: [u8; 256]`，索引为字节值，存储对应数字值 (0-35) 或 `0xFF`(无效)。O(1) 字节到数值映射，无分支。

2. **两阶段累加**:
   - **快路径** (`u32`): 对短整数使用 32-bit 累加器，检查条件 `x <= UINT_MAX/10-1`
   - **慢路径** (`u64`): 当 32-bit 即将溢出时切换到 64-bit 累加器

3. **按基数分派**:
   - `base == 10`: 十进制快路径 (`c - b'0'`)
   - `!(base & (base - 1))`: 2 的幂，使用移位 (`x << shift | val[c]`)
   - 其他: 通用乘法路径

4. **无分支取负**: `(y ^ neg) - neg` 实现条件取反(利用二进制补码性质)

---

#### 不变量 (Invariants)

1. **字符回溯**: 无论成功/失败，第一个非数字字符总是通过 `shunget(f)` 放回流中，保证流位置精确。
2. **errno 只写不读**: `__intscan` 只在错误时设置 `errno`，成功时不修改它。
3. **溢出语义保持**: `(lim & 1)` 标志控制溢出返回值方向，确保调用方能区分 `LLONG_MAX` 正常值和溢出。

---

## 实现指南 (rusl/Rust)

- 查找表使用 `static TABLE: [u8; 256]`，编译期初始化，利用 Rust 的 `const` 求值
- 快/慢路径可以使用 Rust 泛型或 `macro_rules!` 根据目标类型在编译期选择累加器宽度
- 流 I/O 操作 (`shgetc`/`shunget`) 使用内部 `FILE` 结构体的方法(非 trait 对象，避免动态分发开销)
- 错误码使用 `errno` 模块的全局线程局部变量(兼容 C 语义)
- 所有内部辅助函数标记为 `pub(crate)` 或更小可见性