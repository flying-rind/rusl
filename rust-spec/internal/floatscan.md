# floatscan.rs 规约

> **复杂度等级**: Level 1（简单模块 — 仅声明一个内部函数，前置/后置条件即可描述）

---

## 依赖图

```
(外部) 无标准头依赖 ──> floatscan 模块
                         │
                         └── floatscan (pub(crate) 函数)
```

本模块是 rusl 的浮点数扫描（字符串到浮点数转换）子系统的内部模块。它仅导出一个函数 `floatscan`，该函数是 `scanf`、`fscanf`、`sscanf` 以及 `strtod`/`strtof`/`strtold` 系列函数的关键内部实现组件。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ffi` | Rust core | 可用（`#![no_std]` 兼容） |
| 大整数运算 | 内部实现或 `num-bigint` (#![no_std] 兼容) | 浮点解析中的多精度算法 |

---

## 符号规约

---

### `floatscan`

```rust
// Rust 声明 (rusl)
pub(crate) fn floatscan(
    input: &[u8],
    prec: usize,
    pok: u32,
) -> (f64, usize);
```

```c
// C 等价声明 (musl)
hidden long double __floatscan(FILE *, int, int);
```

[Visibility]: Internal — rusl 内部浮点解析函数，POSIX/C 标准未定义。用户程序通过 `scanf("%f")` 或 `strtod()` 等公共 API 间接使用。

#### 功能意图 (Intent)

从给定的字节切片中读取字符序列并将其解析为 `f64` 浮点数。该函数是 rusl 中所有浮点扫描操作的**统一后端**：

- `scanf("%f", &f)` 调用 `floatscan` → 结果截断为 `f32`
- `scanf("%lf", &d)` 调用 `floatscan` → 结果返回 `f64`
- `strtod(s, &end)` 调用 `floatscan` → 直接传入 `&[u8]` 切片
- `strtof` / `strtold` 同理

**统一后端策略**: 所有浮点扫描路径使用 `f64` 作为内部精度，然后将结果向下转换为目标类型。这避免了三份相同解析逻辑的代码重复，同时保证各精度之间的结果一致性。

**与 C 版本的差异**: C 版本使用 `FILE*` 流抽象和 `long double` 精度。Rust 版本直接使用 `&[u8]` 字节切片（无需 `FILE*` 包装），精度固定为 `f64`（Rust 在 `#![no_std]` 下不使用 `f80`/`f128` 扩展精度）。若需要 `f32` 精度，由调用者负责截断。

#### 前置条件 (Preconditions)

- **PRE-1**: `input` 是一个非空字节切片，包含待解析的浮点数字符序列。
- **PRE-2**: `prec` 指定精度控制：
  - 对于 `scanf` 系列：来自格式字符串的精度字段（如 `%10f` 中的 `10`），`0` 表示无限精度。
  - 对于 `strtod` 系列：通常为 `0` 或特定精度要求。
- **PRE-3**: `pok` 是一个标志字（bitmask），指定解析选项：
  - 位 0: 是否允许十六进制浮点数（`0x` / `0X` 前缀）
  - 位 1: 是否允许无穷大/NaN（`inf` / `nan` 字面量）
  - 其他位: 架构/平台特定的解析选项
- **PRE-4**: `input` 中当前位置之后的字符序列应为合法的浮点数字符串，至少包含一个数字字符。

#### 后置条件 (Postconditions)

- **Case 1 (成功解析)**:
  - **POST-1**: 返回 `(value, consumed)` 元组，其中 `value` 为解析后的浮点数值（`f64` 精度），`consumed` 为消费的字节数。
  - **POST-2**: 返回的 `consumed` 值精确对应浮点数字符串结束后的第一个字节偏移（即消费了完整的浮点数字面量）。
  - **POST-3**: 调用者负责根据返回值判断是否需要设置 `errno = ERANGE`（上溢 → `f64::INFINITY`，下溢 → `0.0`）。

- **Case 2 (匹配失败--无数字字符)**:
  - **POST-1**: 返回 `(0.0, 0)`，`consumed == 0` 表示未消费任何输入。
  - **POST-2**: 上层调用者（`scanf`）将此视为**匹配失败**，不消耗任何输入。

- **Case 3 (解析到不完整的数值)**:
  - **POST-1**: 若文本以有效数字开头但中途遇到非法字符，返回已解析部分的值。
  - **POST-2**: `consumed` 停在第一个非法字符之前的字节偏移。

#### 不变量 (Invariants)

- **INV-1 (精度单调性)**: 对于相同的输入字符串，`floatscan` 返回的 `f64` 值在截断为 `f32` 后，必须给出与直接以 `f32` 精度解析相同的结果。
- **INV-2 (往返保证)**: 对于能精确表示的十进制数，`floatscan` 的结果在反向格式化后应能还原原始字符串的有效数字。
- **INV-3 (分离关注)**: `floatscan` 不负责设置 `errno` 或在 `scanf` 语义下的赋值抑制（`%*f`），这些逻辑由上层调用者处理。
- **INV-4 (输入不可变)**: `floatscan` 不修改 `input` 切片本身，通过返回消费字节数让调用者自行推进读取位置。
- **INV-5 (无状态)**: `floatscan` 必须是无状态且可重入的，不依赖全局或静态变量。

#### 边缘情况

| 输入 | 行为 |
|------|------|
| 空切片（`input.is_empty()`） | 返回 `(0.0, 0)`（匹配失败） |
| `"+."` | 作为无数字的非法输入处理（返回 `(0.0, 0)`） |
| `"inf"` / `"infinity"` | 若 `pok` 允许，返回 `(f64::INFINITY, n)` |
| `"nan"` / `"nan(...)"` | 若 `pok` 允许，返回 `(f64::NAN, n)` |
| `"0x1.0p3"` | 若 `pok` 允许十六进制，返回 `(8.0, 8)` |
| 超出 `f64` 范围 | 返回 `(f64::INFINITY, n)`（上溢）或 `(0.0, n)`（下溢） |
| 精度截断（`prec > 0`） | 仅读取 `prec` 个有效字符（用于 `scanf` 宽度限制） |

---

## 全局不变量

- **GINV-1**: `floatscan` 的可见性必须为 `pub(crate)`，不对外导出。
- **GINV-2**: `floatscan` 的实现是 rusl 中较大的模块之一，包含约 500+ 行的浮点解析逻辑，依赖于三个模块私有的辅助函数（`scanexp`、`decfloat`、`hexfloat`）和基于大整数编码的多精度算法。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `floatscan` 实现 | `src/internal/floatscan.rs` | 核心浮点解析器实现 |
| `scanf` 系列 | `src/stdio/scanf.rs` 等 | 调用方 — 格式字符串解析后分派到 `floatscan` |
| `strtod` / `strtof` | `src/stdlib/strtod.rs` 等 | 调用方 — 直接传入 `&[u8]` 切片 |

---

## Rust 实现注意事项 (`#![no_std]`)

1. **`#![no_std]`**: 不能使用 `std::io::Read` 或 `std::fmt`，输入必须是 `&[u8]` 字节切片。不依赖 `FILE*` 流抽象。
2. **精度**: rusl 仅支持 `f32` 和 `f64`（Rust 在 `#![no_std]` 下不原生支持 `f80`/`f128` 扩展精度浮点）。
3. **解析库选型**:
   - `lexical-core` crate: 支持 `#![no_std]`，高度优化的浮点字符串解析。
   - `fast-float` crate: 基于 C++ `fast_float` 库的 Rust 移植。
   - 手工实现: 参考 musl `__floatscan` 的内部算法（基于大整数多精度编码的 Dragon4/Grisu 变体）。
4. **`pok` 标志**: 可以使用 Rust `bitflags!` 宏定义的类型安全标志集。
5. **unsafe 使用**: 浮点解析逻辑完全可以在安全 Rust 中实现，不需要 `unsafe`。