# floatscan.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/floatscan.h`
> **复杂度等级**: Level 1（简单模块 — 仅声明一个内部函数，前置/后置条件即可描述）

---

## 依赖图

```
(外部) <stdio.h> ──> floatscan.h ──> 使用者（scanf 系列函数的浮点解析路径）
                         │
                         └── __floatscan (hidden 函数声明)
```

本文件是 musl 的浮点数扫描（字符串到浮点数转换）子系统的内部头文件。它仅声明一个函数 `__floatscan`，该函数是 `scanf`、`fscanf`、`sscanf` 以及 `strtod`/`strtof`/`strtold` 系列函数的关键内部实现组件。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<stdio.h>` | C 标准库 | 跳过 — 提供 `FILE` 类型定义，`__floatscan` 的参数需要 `FILE *` |
| `hidden` 宏 | `features.h`（通过编译时的隐式包含） | 可见性控制 |

---

## 符号规约

---

### `__floatscan`

```c
hidden long double __floatscan(FILE *, int, int);
```

[Visibility]: Internal — musl 内部浮点解析函数，POSIX/C 标准未定义。用户程序通过 `scanf("%f")` 或 `strtod()` 等公共 API 间接使用。

#### 功能意图 (Intent)

从给定的 `FILE` 流中读取字符序列并将其解析为 `long double` 浮点数。该函数是 musl 中所有浮点扫描操作的**统一后端**：

- `scanf("%f", &f)` 调用 `__floatscan` -> 结果截断为 `float`
- `scanf("%lf", &d)` 调用 `__floatscan` -> 结果截断为 `double`
- `scanf("%Lf", &ld)` 调用 `__floatscan` -> 直接返回 `long double`
- `strtod(s, &end)` 调用 `__floatscan` -> 通过 `fmemopen` 或等效方式包装字符串为 `FILE` 流
- `strtof` / `strtold` 同理

**统一后端策略**: 所有浮点扫描路径使用 `long double` 作为内部精度，然后将结果向下转换为目标类型。这避免了三份相同解析逻辑的代码重复，同时保证各精度之间的结果一致性。

#### 前置条件 (Preconditions)

- **PRE-1**: 第一个参数 `f` 是一个有效的、已打开的 `FILE *` 流（可以是真实文件流，也可以是通过 `fmemopen` 创建的内存流），其读取位置指向待解析的浮点数字符串的开始位置。
- **PRE-2**: 第二个参数 `prec` 指定精度控制：
  - 对于 `scanf` 系列：来自格式字符串的精度字段（如 `%10f` 中的 `10`），0 表示无限精度。
  - 对于 `strtod` 系列：通常为 0 或特定精度要求。
- **PRE-3**: 第三个参数 `pok` 是一个标志字（bitmask），指定解析选项：
  - 位 0: 是否允许十六进制浮点数（`0x` / `0X` 前缀）
  - 位 1: 是否允许无穷大/NaN（`inf` / `nan` 字面量）
  - 其他位: 架构/平台特定的解析选项

（注意：具体的 `pok` 位定义可能因 musl 版本和架构而异，以上为典型语义。）

- **PRE-4**: 流中当前位置之后的字符序列应为合法的浮点数字符串，至少包含一个数字字符。

#### 后置条件 (Postconditions)

- **Case 1 (成功解析)**:
  - **POST-1**: 返回解析后的浮点数值（`long double` 精度）。
  - **POST-2**: `FILE` 流的读取位置前进到浮点数字符串结束后的第一个字符（即消费了完整的浮点数字面量）。
  - **POST-3**: `errno` 可能被设置为 `ERANGE` 如果结果溢出（返回 `HUGE_VALL`）或下溢到零。
  - **POST-4**: 函数本身不读取 `f` 的 `errno`，`errno` 的最终设置由上层调用者（如 `scanf`、`strtod`）处理。

- **Case 2 (匹配失败--无数字字符)**:
  - **POST-1**: 返回 0.0。
  - **POST-2**: `FILE` 流的读取位置**不变**（通过 `unget` 将已读取的非匹配字符退回）。
  - **POST-3**: 上层调用者（`scanf`）将此视为**匹配失败**，不消耗任何输入。

- **Case 3 (解析到不完整的数值)**:
  - **POST-1**: 若文本以有效数字开头但中途遇到非法字符，返回已解析部分的值。
  - **POST-2**: `FILE` 流的读取位置停在第一个非法字符之前。

#### 不变量 (Invariants)

- **INV-1 (精度单调性)**: 对于相同的输入字符串，`__floatscan` 返回的 `long double` 值在截断为 `float` 或 `double` 后，必须给出与直接以该精度解析相同的结果（即结果一致性：`(float)__floatscan(f, prec, pok)` 与独立 float 解析器结果一致）。
- **INV-2 (往返保证)**: 对于能精确表示的十进制数，`__floatscan` 的结果在反向格式化（`printf`）后应能还原原始字符串的有效数字。
- **INV-3 (分离关注)**: `__floatscan` 不负责设置 `errno` 或在 `scanf` 语义下的赋值抑制（`%*f`），这些逻辑由上层调用者处理。
- **INV-4 (流位置)**: 在成功解析后，流位置必须精确停在浮点数表示结束后的字符（通常为空白、分隔符或非数字字符），不能多消费也不能少消费。
- **INV-5 (线程安全)**: `__floatscan` 必须是无状态且可重入的，因为它被 `scanf` 和 `strtod` 系列从多个线程同时调用。其所有状态存储在 `FILE *` 流和局部变量中，不依赖全局或静态变量。

#### 边缘情况

| 输入 | 行为 |
|------|------|
| 空流（EOF at start） | 返回 0，流位置不变（匹配失败） |
| `"+."` | 作为无数字的非法输入处理（返回 0，流位置不变） |
| `"inf"` / `"infinity"` | 若 `pok` 允许，返回 `INFINITY`；否则为非法输入 |
| `"nan"` / `"nan(...)"` | 若 `pok` 允许，返回 `NAN`（可能带指定载荷） |
| `"0x1.0p3"` | 若 `pok` 允许十六进制，返回 8.0 |
| 超出 `long double` 范围 | 返回 `HUGE_VALL`（上溢）或 0（下溢），`errno = ERANGE` |
| 精度截断（`prec > 0`） | 仅读取 `prec` 个有效字符（用于 `scanf` 宽度限制） |

---

## 全局不变量

- **GINV-1**: 符号 `__floatscan` 的可见性必须为 `hidden`，确保动态链接时不会被用户程序意外覆盖或符号冲突。
- **GINV-2**: `__floatscan` 的实现（位于 `src/internal/floatscan.c`）是 musl 中最大的单文件之一，包含约 500+ 行的浮点解析逻辑，依赖三个 `static` 辅助函数（`scanexp`、`decfloat`、`hexfloat`）和基于 B1B (base 10^9) 编码的多精度算法。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `__floatscan` 实现 | `src/internal/floatscan.c` | 核心浮点解析器实现 |
| `scanf` 系列 | `src/stdio/scanf.c` 等 | 调用方 -- 格式字符串解析后分派到 `__floatscan` |
| `strtod` / `strtof` / `strtold` | `src/stdlib/strtod.c` 等 | 调用方 -- 通过 `fmemopen` 包装 C 字符串为 `FILE` |
| `__shlim` / `__shgetc` 等 | `src/stdio/` | `FILE` 流的底层缓冲/读取原语（`__floatscan` 内部使用） |

---

## Rust 实现提示 (`#![no_std]`)

在 `rusl` 中，`__floatscan` 的功能对应为浮点解析模块：

```rust
// 声明为 pub(crate)，不可对外导出
pub(crate) fn floatscan(
    input: &[u8],          // 替代 FILE * 流
    prec: usize,           // 精度
    pok: u32,              // 解析选项标志
) -> (f64, usize) {        // 返回 (解析值, 消费的字节数)
    // 使用 minimal-lexical 或手工实现的浮点解析
    // 或使用 lexical-core crate (支持 #![no_std])
    todo!()
}
```

**关键考量**:

1. **`#![no_std]`**: 不能使用 `std::io::Read` 或 `std::fmt`，输入必须是 `&[u8]` 字节切片。
2. **精度**: `rusl` 可能仅支持 `f32` 和 `f64`（Rust 不原生支持 `long double`即 `f80`/`f128`）。
3. **解析库选型**:
   - `lexical-core` crate: 支持 `#![no_std]`，高度优化的浮点字符串解析。
   - `fast-float` crate: 基于 C++ `fast_float` 库的 Rust 移植。
   - 手工实现: 参考 musl `__floatscan` 的内部算法（基于 B1B 多精度编码的 Dragon4/Grisu 变体）。
4. **`FILE *` 流替代**: 在纯 Rust 实现中，不需要 `FILE *` 抽象。`scanf` 和 `strtod` 直接传递 `&[u8]` 切片及其当前位置。
5. **`pok` 标志**: 可以转为 Rust `bitflags!` 宏定义的类型安全标志集。