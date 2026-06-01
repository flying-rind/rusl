# complex_impl 模块规约 (Rust)

> **源 C spec**: `/home/mangp/桌面/OS/musl/src/internal/spec/complex_impl.md`
> **复杂度等级**: Level 1（简单模块 — 类型别名 + 内部函数声明）

---

## 依赖图

```
(外部) core::ops / core::fmt ──> complex_impl 模块
                                     │
                                     ├── Complex32 / Complex64 类型定义
                                     ├── CMPLX / CMPLXF / CMPLXL 构造宏
                                     ├── __ldexp_cexp (内部函数声明)
                                     └── __ldexp_cexpf (内部函数声明)
```

本模块为 rusl 数学库的复数运算模块提供基础设施：复数类型定义、构造宏和两个内部复数函数声明。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `core::ops::{Add, Sub, Mul, Div, Neg}` | Rust core 库 | 复数运算 trait 实现 |
| `core::fmt` | Rust core 库 | Debug/Display trait 实现（可选） |

---

## 复数类型设计

由于 Rust 语言没有内建复数类型（与 C 的 `_Complex` 关键字不同），rusl 内部需要自定义 `Complex32`（对应 `float complex`）和 `Complex64`（对应 `double complex`）类型。`long double complex` 在 musl 对应平台上为 80 位 x87 扩展精度或 128 位四倍精度，Rust 中可定义 `Complex80` 或 `Complex128` 类型（依平台而定）。

```rust
// Rust — 复数类型定义
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Complex32 {
    pub re: f32,  // 实部
    pub im: f32,  // 虚部
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Complex64 {
    pub re: f64,  // 实部
    pub im: f64,  // 虚部
}

// 注意：Complex80/Complex128 需根据 target_pointer_width 和 target_arch 条件编译定义
// 在 x86_64 上，long double = 80-bit x87 extended precision
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Complex80 {
    pub re: f80,  // 实部（需自定义 f80 类型或使用裸字节数组 [u8; 10] + 填充 [u8; 6]）
    pub im: f80,  // 虚部
}
```

---

## 符号规约

---

### `Complex32::new` / `Complex64::new`

```rust
// Rust 声明 — 复数构造
impl Complex32 {
    pub(crate) fn new(re: f32, im: f32) -> Self;
}

impl Complex64 {
    pub(crate) fn new(re: f64, im: f64) -> Self;
}
```

[Visibility]: Internal — rusl 数学库内部类型方法，等价于 C 的 `CMPLXF` / `CMPLX` 宏。

#### 功能意图 (Intent)

用两个标量值构造复数。Rust 版本通过关联函数 `new` 实现，替代 C 的宏和联合类型别名技巧。编译器将内联该函数，实现零开销。

#### 前置条件 (Preconditions)

- **PRE-1**: `re` 和 `im` 为任意有效的 IEEE 754 浮点值（包括 NaN、Inf、次正规数）。

#### 后置条件 (Postconditions)

- **POST-1**: 返回一个复数，其实部 = `re`，虚部 = `im`。
- **POST-2**: `Complex32::new(x, 0.0).im` 精确为 `+0.0f32`（非 `-0.0`）。

#### 不变量 (Invariants)

- **INV-1**: `Complex32`/`Complex64` 的 `#[repr(C)]` 布局保证与 C `_Complex float`/`_Complex double` 的内存布局兼容（实部在前、虚部在后、各占 sizeof(T) 字节）。
- **INV-2**: 该布局不依赖 C11 复合字面量技巧，仅依赖 Rust 的 `#[repr(C)]` 保证。

---

### `CMPLXF` / `CMPLX` / `CMPLXL` 宏

```rust
// Rust 声明 — 等效宏（保持与 C musl 代码相同的调用风格）
#[macro_export] // 仅在 crate 内部使用
macro_rules! CMPLXF {
    ($x:expr, $y:expr) => { crate::internal::complex_impl::Complex32::new($x as f32, $y as f32) };
}

macro_rules! CMPLX {
    ($x:expr, $y:expr) => { crate::internal::complex_impl::Complex64::new($x as f64, $y as f64) };
}

// CMPLXL 的宏定义依平台而定
// x86_64: Complex80::new($x as f80, $y as f80)
// aarch64: Complex128::new($x as f128, $y as f128)
```

[Visibility]: Internal — rusl 内部宏，等价于 musl C 的 `CMPLXF`/`CMPLX`/`CMPLXL` 宏。

#### 功能意图 (Intent)

提供与 musl C 代码相同的复数构造语法糖，降低从 C 代码迁移的心智负担。内部复数的各个数学函数（`csqrt`、`cexp` 等）可直接使用 `CMPLX(re, im)` 宏。

#### 后置条件 (Postconditions)

- **POST-1**: `CMPLXF(x, y)` 展开为 `Complex32::new(x as f32, y as f32)`。
- **POST-2**: `CMPLX(x, y)` 展开为 `Complex64::new(x as f64, y as f64)`。

---

### `__ldexp_cexp`

```rust
// Rust 声明 — 内部函数，不对外导出
pub(crate) fn __ldexp_cexp(z: Complex64, n: c_int) -> Complex64;
```

[Visibility]: Internal — rusl 数学库内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

计算 `ldexp(cexp(z), n)` 的组合操作，即 `exp(z) * 2^n`。通过**融合乘幂操作**避免了先计算 `exp(z)` 再乘 `2^n` 可能导致的中间溢出/下溢问题——当 `exp(z)` 超出 double 表示范围但 `exp(z) * 2^n` 在范围内时，单独计算会导致错误。

#### 前置条件 (Preconditions)

- **PRE-1**: `z` 为任意有效的 `Complex64` 值（包括 NaN、Inf、次正规数）。
- **PRE-2**: `n` 为整数指数。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `exp(z) * 2^n`（在 `Complex64` 精度范围内）。
- **POST-2**: 若结果在 `f64` 范围内可表示，即使中间结果 `exp(z)` 单独不可表示，函数也能正确返回（无虚假溢出/下溢）。

#### 不变量 (Invariants)

- **INV-1**: 对于任意 `z` 和 `n`，`__ldexp_cexp(z, n) == ldexp(cexp(z), n)` 在数学上成立（但前者在边界情况下更精确）。
- **INV-2**: 该函数是 `cexp` 实现的关键内部组件，不应被外部代码直接调用。

#### Rust 实现建议

在 rusl 中，该函数内部可通过手动拆分实部和虚部并分别对指数进行缩放来实现：
- 先将复数 `z` 拆为 `re + i*im`
- 使用 `exp(re) * cos(im)` 和 `exp(re) * sin(im)` 公式
- 在计算过程中结合 `2^n` 缩放以避免中间溢出

---

### `__ldexp_cexpf`

```rust
// Rust 声明 — 内部函数，不对外导出
pub(crate) fn __ldexp_cexpf(z: Complex32, n: c_int) -> Complex32;
```

[Visibility]: Internal — rusl 数学库内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

`__ldexp_cexp` 的 `f32` 版本。计算 `ldexpf(cexpf(z), n)` 的融合操作。

#### 前置条件 (Preconditions)

- **PRE-1**: `z` 为任意有效的 `Complex32` 值。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `exp(z) * 2^n`（在 `Complex32` 精度范围内）。
- **POST-2**: 避免中间溢出/下溢。

---

## 全局不变量

- **GINV-1**: 所有 `Complex32`/`Complex64`/`Complex80` 类型使用 `#[repr(C)]` 布局，确保实部在前、虚部在后，与 C ABI 兼容。
- **GINV-2**: `complex_impl` 中所有 `pub(crate)` 函数仅对 rusl crate 内部可见，外部程序无法链接到这些符号。
- **GINV-3**: 复数类型的 `re` 和 `im` 字段为 `pub`（模块内部可见），允许数学函数直接访问以进行性能优化。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `Complex32` / `Complex64` | 本模块 | 复数类型定义 |
| `__ldexp_cexp` 实现 | `src/complex/ldexp_cexp.rs` | 实现位于独立文件 |
| `__ldexp_cexpf` 实现 | `src/complex/ldexp_cexpf.rs` | 实现位于独立文件 |
| `cexp` (公共 API) | `src/complex/cexp.rs` | `cexp` 内部调用 `__ldexp_cexp` |
| `f80` 类型 | 本模块或 `src/internal/float.rs` | x86_64 扩展精度浮点类型定义 |

---

## Rust 实现提示

在 `rusl` (`#![no_std]`) 中，复数类型完全自定义实现：

| 特性 | 实现方式 |
|------|----------|
| 复数加法 | 实现 `core::ops::Add` trait |
| 复数乘法 | 实现 `core::ops::Mul` trait（使用 FOIL 公式） |
| 复数除法 | 实现 `core::ops::Div` trait（使用 Smith 算法避免溢出） |
| 复数 abs | `re.hypot(im)`（利用 `f32::hypot` / `f64::hypot` 内建方法） |
| 复数 exp | 基于 `re.exp() * im.cos()` 和 `re.exp() * im.sin()` |
| `f80` 类型 | 若目标平台为 x86_64，需自定义 80 位浮点类型；可通过 `#[repr(C)]` 结构体包装 `[u8; 10]`（含 6 字节尾部填充到 16 字节对齐），并在关键运算中使用内联 x87 汇编 |