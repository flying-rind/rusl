# intscan.h 规约

## 概述

`intscan.h` 声明了 musl 内部的整数流解析函数 `__intscan`。该函数是 `scanf` 系列和 `strtol`/`strtoul` 系列函数的底层解析引擎，从 `FILE *` 流中按指定基数读取整数字符序列。头文件仅包含一个函数声明，实现位于 `src/internal/intscan.c`。

## 依赖图

```
intscan.h
├── <stdio.h>          (标准库头文件, 提供 FILE 类型声明)
│
└── __intscan          (声明于本文件)
    └── 实现依赖:
        ├── src/internal/intscan.c       (实现所在文件)
        ├── src/internal/shgetc.h        (musl 内部, shgetc/shunget/shlim 宏)
        ├── <limits.h>                   (标准库, UINT_MAX/ULLONG_MAX)
        ├── <errno.h>                    (标准库, EINVAL/ERANGE)
        └── <ctype.h>                    (标准库, isspace)
```

## 类型依赖

| 依赖类型 | 来源 | 处理方式 |
|---------|------|----------|
| `FILE` | `<stdio.h>` | 标准库类型，跳过 |
| `unsigned` | C 内建类型 | 跳过<｜image｜>|
| `int` | C 内建类型 | 跳过 |
| `unsigned long long` | C 内建类型 | 跳过 |

---

## 符号规约

### __intscan

```c
hidden unsigned long long __intscan(FILE *f, unsigned base, int pok, unsigned long long lim);
```
```rust
// Rust
unsafe fn __intscan(
    f: *mut FILE,
    base: c_uint,
    pok: c_int,
    lim: c_ulonglong,
) -> c_ulonglong;
```

[Visibility]: Internal (不导出) — musl 内部整数解析引擎。被 `strtol`/`strtoul`/`strtoll`/`strtoull` (`src/stdlib/strtol.c`) 和 `vfscanf` (`src/stdio/vfscanf.c`) 调用。POSIX/C 标准未定义此符号。

**Intent**: musl 将所有文本到整数的转换统一到一个函数中。`strtol`/`strtoul` 通过创建内存 FILE 流并调用 `__intscan` 实现；`scanf` 的 `%d`/`%i`/`%u`/`%x`/`%o` 转换也委托给 `__intscan`。这种设计避免了每个整数类型和每种输入源组合都独立实现的代码膨胀。

---

#### 参数契约

| 参数 | 契约 |
|------|------|
| `f` | 指向已初始化的 FILE 流。调用前应由 `shlim(f, width)` 设置读取宽度限制 |
| `base` | 进制基数。合法值：`0`（自动检测）、`[2, 36]`。`base == 1` 或 `base > 36` 非法 |
| `pok` | "peek only" 标志。`0` = 消费模式（正常解析），非零 = 仅窥探模式（失败时可回溯全部已读字符） |
| `lim` | 溢出上限。最低位 (`lim & 1`) 有特殊语义：`1` = 无符号溢出方向（允许 nagative overflow），`0` = 有符号溢出方向 |

---

#### 前置条件 (Pre-conditions)

1. `f != NULL` — 流指针有效且内部缓冲区已初始化
2. `f` 已由调用者通过 `shlim(f, width)` 设置读取宽度限制（来自 `scanf` 的 field width 或 `strtol` 的字符串长度）
3. `base` ∈ {`0`} ∪ [`2`, `36`]
4. `lim` 设置为对应目标类型的溢出上限值：
   - `unsigned int`: `lim = UINT_MAX`, 带 `(lim & 1) == 1` 语义
   - `signed long long`: `lim = (unsigned long long)LLONG_MAX + 1`, 带 `(lim & 1) == 0` 语义

---

#### 后置条件 (Post-conditions) — 按返回值案例分析

**Case A — 成功解析**:
- 返回值 = 解析出的整数值（`unsigned long long` 类型，调用者可按需转换为有符号类型）
- `errno` 不被修改（保留调用前的值）
- 流位置前进至第一个非数字字符

**Case B — 无有效数字字符**:
- 返回 `0`
- `errno = EINVAL`
- 流位置不变或已调用 `shlim(f, 0)` 终止读取

**Case C — 溢出**:
- 返回 `lim`（正溢出）或 `lim-1`（负溢出/无符号溢出）
- `errno = ERANGE`
- 所有溢出数字已被消费，流位置前进至第一个非数字字符

**Case D — 非法基数** (`base > 36 || base == 1`):
- 返回 `0`
- `errno = EINVAL`

---

#### System Algorithm（Level 3 — 高度优化）

`__intscan` 采用**分层累加 + 查表 + 按基分派**策略：

1. **256 字节查找表**: 静态 `const unsigned char table[256]`，索引为 ASCII 字符值，存储对应数字值 (0-35) 或 0xFF（无效）。O(1) 字符→数值映射，无分支。

2. **两阶段累加**:
   - **快路径** (`unsigned int`): 对短整数使用 32-bit 累加器，检查条件 `x <= UINT_MAX/10-1`
   - **慢路径** (`unsigned long long`): 当 32-bit 即将溢出时切换到 64-bit 累加器

3. **按基数分派**:
   - `base == 10`: 十进制快路径 (`c - '0'`)
   - `!(base & base-1)`: 2 的幂，使用移位 (`x << bs | val[c]`)
   - 其他: 通用乘法路径

4. **无分支取负**: `(y ^ neg) - neg` 实现条件取反

---

#### 不变量 (Invariants)

1. **字符回溯**: 无论成功/失败，第一个非数字字符总是通过 `shunget(f)` 放回流中，保证流位置精确。
2. **errno 只写不读**: `__intscan` 只在错误时设置 `errno`，成功时不修改它。
3. **溢出语义保持**: `(lim & 1)` 标志控制溢出返回值方向，确保调用方能区分 `LLONG_MAX` 正常值和溢出。