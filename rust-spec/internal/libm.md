# libm 规约 (Rust)

## 概述

`libm` 模块是 rusl 数学库的核心内部模块。它定义了浮点数底层操作所需的类型、位操作工具函数、以及大量内部数学辅助函数的声明(三角函数归约、指数函数、错误处理等)。该模块被 rusl `src/math/` 下几乎所有源文件引用。

## 依赖图

```
libm 模块
├── [条件编译类型]
│   └── union LdShape    (四种变体按平台选择, 定义于本模块)
│
├── [内联函数]
│   ├── eval_as_float / eval_as_double
│   ├── fp_barrierf / fp_barrier / fp_barrierl (若架构未提供则定义)
│   └── fp_force_evalf / fp_force_eval / fp_force_evall (若架构未提供则定义)
│   └── FORCE_EVAL 宏
│
├── [类型双关函数]
│   ├── asuint / asfloat / asuint64 / asdouble
│   ├── extract_words / get_high_word / get_low_word
│   └── insert_words / set_high_word / set_low_word
│   └── get_float_word / set_float_word
│
├── [内部函数 — 三角函数归约 (f64)]
│   ├── rem_pio2_large(x, y, e0, nx, prec)        → src/math/__rem_pio2_large.rs
│   ├── rem_pio2(x, y)                              → src/math/__rem_pio2.rs
│   ├── __sin(x, y, n)                              → src/math/__sin.rs
│   ├── __cos(x, y)                                 → src/math/__cos.rs
│   ├── __tan(x, y, n)                              → src/math/__tan.rs
│   └── expo2(x, sign)                              → src/math/__expo2.rs
│
├── [内部函数 — 三角函数归约 (f32)]
│   ├── rem_pio2f(x, y)                             → src/math/__rem_pio2f.rs
│   ├── sindf(x)                                    → src/math/__sindf.rs
│   ├── cosdf(x)                                    → src/math/__cosdf.rs
│   ├── tandf(x, n)                                 → src/math/__tandf.rs
│   └── expo2f(x, sign)                             → src/math/__expo2f.rs
│
├── [内部函数 — 三角函数归约 (f128)]
│   ├── rem_pio2l(x, y)                             → src/math/__rem_pio2l.rs
│   ├── __sinl(x, y, n)                             → src/math/__sinl.rs
│   ├── __cosl(x, y)                                → src/math/__cosl.rs
│   └── __tanl(x, y, n)                             → src/math/__tanl.rs
│
├── [内部函数 — 多项式求值 (f128)]
│   ├── polevll(x, coef, n)                         → src/math/__polevll.rs
│   └── p1evll(x, coef, n)                          → src/math/__p1evll.rs
│
├── [内部函数 — lgamma / signgam]
│   ├── lgamma_r(x, signgamp)                       → src/math/__lgamma_r.rs
│   ├── lgammaf_r(x, signgamp)                      → src/math/__lgammaf_r.rs
│   └── signgam (全局变量 + 公开导出 signgam)       → src/math/__signgam.rs
│
└── [内部函数 — 错误处理]
    ├── math_xflowf(sign, y) / math_uflowf(sign) / math_oflowf(sign) / math_divzerof(sign) / math_invalidf(x)
    ├── math_xflow(sign, y) / math_uflow(sign) / math_oflow(sign) / math_divzero(sign) / math_invalid(x)
    └── math_invalidl(x) [条件: LDBL_MANT_DIG != DBL_MANT_DIG]
```

---

```
/* Rely */
[RELY]
架构依赖:
  fp_barrierf / fp_barrier / fp_barrierl          // 依赖1: 架构特定编译器屏障(可选, 未提供时使用回退实现)
  fp_force_evalf / fp_force_eval / fp_force_evall // 依赖2: 架构特定浮点求值强制器(可选)

标准依赖:
  core::f32::to_bits / core::f32::from_bits        // 依赖3: Rust 内建浮点位转换
  core::f64::to_bits / core::f64::from_bits        // 依赖4: Rust 内建浮点位转换
  core::hint::black_box                            // 依赖5: 编译器优化屏障(回退实现)

内部依赖:
  math_errhandling 常量                            // 依赖6: math 错误处理模式

[GUARANTEE]
外部接口:
  extern "C" static mut signgam: c_int;            // POSIX 公开, lgamma 符号输出

内部接口:
  pub(crate) fn eval_as_float(x: f32) -> f32;      // 本模块保证: 精度截断
  pub(crate) fn eval_as_double(x: f64) -> f64;     // 本模块保证: 精度截断
  pub(crate) fn asuint(f: f32) -> u32;             // 本模块保证: 逐位等价转换
  pub(crate) fn asuint64(f: f64) -> u64;           // 本模块保证: 逐位等价转换
  pub(crate) fn rem_pio2(x: f64, y: &mut [f64; 2]) -> c_int;  // 本模块保证: 高精度 π/2 归约
  ... (其余内部函数按各自规约)
```

---

## 第一部分: 条件编译类型定义

### LdShape (四种变体)

`LdShape` 是一个将 `long double` 的原始字节重新解释为各个组成部分(符号/指数/尾数)的联合体。由于 `long double` 在不同平台上有不同的表示(80-bit x87 扩展精度、128-bit IEEE 754 四倍精度)，定义了四种变体。

---

#### 变体 1: IEEE 754 double == long double (无 LdShape)

```rust
// cfg: LDBL_MANT_DIG == 53 && LDBL_MAX_EXP == 1024
// LdShape 未定义 — long double 与 f64 完全相同, 所有操作直接使用 f64 类型
```

[Visibility]: Internal — 条件编译分支

**适用平台**: `long double` 与 `double` 完全相同的架构(如部分 ARM 平台)。在此情况下，所有 `long double` 操作退化为 `f64` 操作，不需要 `LdShape`。

---

#### 变体 2: 80-bit x87 扩展精度 — 小端序

```rust
// Rust — x86/x86_64 80-bit 扩展精度 (小端序)
// cfg: LDBL_MANT_DIG == 64 && LDBL_MAX_EXP == 16384 && target_endian == "little"

#[repr(C)]
pub(crate) union LdShape {
    pub f: [u8; 10],         // 80-bit x87 扩展精度浮点数(10字节原始表示)
    pub i: LdShape80LE,
}

#[repr(C)]
pub(crate) struct LdShape80LE {
    pub m: u64,    // 尾数 (63 位有效 + 1 位显式整数位)
    pub se: u16,   // 位[15]: 符号, 位[14:0]: 指数 (biased by 16383)
}
```

[Visibility]: Internal (不导出) — rusl libm 内部类型，用于在 x87 80-bit 扩展精度平台上操作 long double 的位表示

**适用平台**: x86 (32-bit), x86_64(使用 x87 FPU 时)

**布局不变量**:
- 总大小: 10 字节(80 位)，但 `size_of::<LdShape>()` 可能为 12 或 16 字节(对齐填充)
- 内存布局(小端序，低地址到高地址): `m[0..7]` (8 字节) → `se[0..1]` (2 字节)
- `se` 的位[15] 为符号位
- `se` 的位[14:0] 为偏置指数(bias = 16383)
- `m` 的位[63] 为显式前导位(整数位)，正常化为 1; subnormal 时为 0

---

#### 变体 3: 80-bit x87 扩展精度 — 大端序 (m68k 变体)

```rust
// Rust — m68k 大端序 80-bit long double
// cfg: LDBL_MANT_DIG == 64 && LDBL_MAX_EXP == 16384 && target_endian == "big"

#[repr(C)]
pub(crate) struct LdShape80BE {
    pub se: u16,    // 符号 + 指数
    pub pad: u16,   // 对齐填充(使 m 在 4 字节边界对齐)
    pub m: u64,     // 尾数
}
```

[Visibility]: Internal (不导出) — m68k 大端序 80-bit long double

**适用平台**: Motorola 68000 系列

**限制**: 仅当 `u64` 对齐要求 ≤ 4 字节时才正确。

---

#### 变体 4: IEEE 754 四倍精度 (128-bit) — 小端序

```rust
// Rust — IEEE 754 128-bit 四倍精度 (小端序)
// cfg: LDBL_MANT_DIG == 113 && LDBL_MAX_EXP == 16384 && target_endian == "little"

#[repr(C)]
pub(crate) union LdShape {
    pub f: f128,
    pub i: LdShape128LE,
    pub i2: LdShape128LE2,
}

#[repr(C)]
pub(crate) struct LdShape128LE {
    pub lo: u64,    // 尾数低 64 位
    pub mid: u32,   // 尾数中 32 位
    pub top: u16,   // 尾数高 16 位
    pub se: u16,    // 1-bit 符号 + 15-bit 指数
}

#[repr(C)]
pub(crate) struct LdShape128LE2 {
    pub lo: u64,
    pub hi: u64,    // hi = se << 48 | top << 32 | mid
}
```

[Visibility]: Internal (不导出) — IEEE 754 128-bit 四倍精度

**适用平台**: aarch64 (部分), POWER9+, RISC-V (Q 扩展)

**不变量**:
- `se` 的位[15] = 符号，位[14:0] = 偏置指数(bias = 16383)
- 尾数 112 位分布: `top`(16) + `mid`(32) + `lo`(64)
- `i2.hi` 将 `se`, `top`, `mid` 打包为一个 `u64`

---

#### 变体 5: IEEE 754 四倍精度 (128-bit) — 大端序

```rust
// Rust — 大端序 128-bit 四倍精度
// cfg: LDBL_MANT_DIG == 113 && LDBL_MAX_EXP == 16384 && target_endian == "big"

#[repr(C)]
pub(crate) struct LdShape128BE {
    pub se: u16,
    pub top: u16,
    pub mid: u32,
    pub lo: u64,
}
```

[Visibility]: Internal (不导出) — 大端序四倍精度

**适用平台**: s390x, 某些 POWER 配置

---

## 第二部分: 内联函数规约

### eval_as_float / eval_as_double

```rust
// Rust — 浮点精度强制截断工具
#[inline]
pub(crate) fn eval_as_float(x: f32) -> f32 {
    // 通过赋值给局部变量强制将值舍入到 f32 精度
    let y: f32 = x;
    y
}

#[inline]
pub(crate) fn eval_as_double(x: f64) -> f64 {
    let y: f64 = x;
    y
}
```

[Visibility]: Internal (不导出) — rusl libm 内部精度控制工具

**Intent (Level 2)**: 消除 x86 平台上 FPU 寄存器 80-bit 额外精度(excess precision)带来的问题。通过赋值给一个显式类型的局部变量，强制将值舍入到声明类型的精度(`f32` -> 32-bit, `f64` -> 64-bit)。

**前置条件**: `x` 为有效浮点值。

**后置条件**: 返回 `x` 被舍入到目标类型精度后的值。返回值按 IEEE 754 规则舍入到 `f32` (24-bit 尾数) 或 `f64` (53-bit 尾数)。

**Invariant**: `eval_as_float(x) == (x as f32) as f32`(编译器可能优化掉多余的转换，但 `eval_as_float` 通过赋值确保至少一次舍入)。

---

### fp_barrierf / fp_barrier / fp_barrierl

```rust
// Rust — 浮点编译器优化屏障 (架构通用回退实现)
#[inline]
pub(crate) fn fp_barrierf(x: f32) -> f32 {
    core::hint::black_box(x)  // 等效于 C: volatile float y = x; return y;
}

#[inline]
pub(crate) fn fp_barrier(x: f64) -> f64 {
    core::hint::black_box(x)
}

// fp_barrierl 同理, 参数类型为 f128 或 [u8; 10]
```

[Visibility]: Internal (不导出) — rusl libm 内部编译器优化屏障

**Intent (Level 3)**: `fp_barrierf` 充当编译器优化屏障——它强制编译器假设函数调用有副作用，从而阻止编译器跨该调用传播关于 `x` 的值的假设。用于:
1. 防止编译器将浮点常量折叠优化过早应用
2. 分隔浮点异常测试的不同阶段
3. 确保舍入模式更改后生效

**优先级**: 若架构特定模块提供了内联汇编版本(如 aarch64 使用 `asm!("", inout(vreg) x)`)，则覆盖此默认实现。汇编版本更精确，因为它不强制实际内存存储而仅阻止寄存器优化。

**前置条件**: `x` 为有效浮点值(含 NaN/Inf)。

**后置条件**: 返回值与 `x` 逐位相同，但编译器必须在调用点完成对 `x` 的所有待定浮点操作。

---

### fp_force_evalf / fp_force_eval / fp_force_evall

```rust
// Rust — 浮点表达式求值强制器 (用于触发浮点异常)
#[inline]
pub(crate) fn fp_force_evalf(x: f32) {
    core::hint::black_box(x);  // 等效于 C: volatile float y; y = x;
}

#[inline]
pub(crate) fn fp_force_eval(x: f64) {
    core::hint::black_box(x);
}
```

[Visibility]: Internal (不导出) — rusl libm 内部浮点表达式求值强制器

**Intent**: 强制编译器对 `x` 进行完整求值，但**不保留结果**。用于触发浮点异常(如 underflow、inexact)，而不影响程序的计算结果。

**典型用法**:
```rust
// 触发 underflow 异常
fp_force_evalf(tiny * tiny);
```

**前置条件**: `x` 为有效浮点值。

**后置条件**: 对 `x` 的求值已完成，可能触发的浮点异常已发生。调用后 `x` 的值被丢弃。无返回值。

---

### FORCE_EVAL 宏

```rust
// Rust — 类型通用浮点求值强制器宏
macro_rules! FORCE_EVAL {
    ($x:expr) => {{
        let v = $x;
        // 根据表达式类型分发到正确的 fp_force_eval* 函数
        // sizeof 判断在编译期由常量折叠消除
        match core::mem::size_of_val(&v) {
            4 => fp_force_evalf(v),
            8 => fp_force_eval(v),
            _ => fp_force_evall(v),
        }
    }};
}
```

[Visibility]: Internal (不导出) — rusl libm 内部类型通用浮点求值强制器

**Intent**: 封装对 `fp_force_evalf` / `fp_force_eval` / `fp_force_evall` 的调用，根据表达式类型自动选择正确的版本。`size_of` 的判断在编译期由编译器优化消除，不会产生运行时开销。

---

## 第三部分: 位操作函数规约

### 类型双关函数

```rust
// Rust — 浮点值与整数位模式的安全零开销转换
// 使用 Rust 内建的 to_bits / from_bits 方法, 无需 unsafe
#[inline]
pub(crate) fn asuint(f: f32) -> u32    { f.to_bits() }

#[inline]
pub(crate) fn asfloat(i: u32) -> f32   { f32::from_bits(i) }

#[inline]
pub(crate) fn asuint64(f: f64) -> u64  { f.to_bits() }

#[inline]
pub(crate) fn asdouble(i: u64) -> f64  { f64::from_bits(i) }
```

[Visibility]: Internal (不导出) — rusl libm 内部浮点位重新解释函数

**Intent**: 使用 Rust 内建的 `to_bits()` / `from_bits()` 方法实现浮点值与整数位模式的零开销安全转换。这等效于 C 中使用匿名联合的类型双关(compound literal + anonymous union)，且完全安全(无 unsafe)。

**前置条件**:
- `asuint(f)`: `f` 为有效 `f32` 值
- `asfloat(i)`: `i` 为任意 32-bit 整数
- `asuint64(f)`: `f` 为有效 `f64` 值
- `asdouble(i)`: `i` 为任意 64-bit 整数

**后置条件**: 返回输入值的逐位等价表示。不存在舍入或精度损失。

**不变量**:
- `asfloat(asuint(f)) == f`(往返恒等式)
- `asdouble(asuint64(f)) == f`(往返恒等式)
- 注意: `asuint(asfloat(i))` 不一定等于 `i`(因为存在 NaN 规范化/signaling NaN 可能变为 quiet NaN)

---

### 双精度位操作函数

```rust
// Rust — 将 f64 拆分为高/低 32 位字
#[inline]
pub(crate) fn extract_words(d: f64) -> (u32, u32) {
    let u = asuint64(d);
    ((u >> 32) as u32, u as u32)
}

#[inline]
pub(crate) fn get_high_word(d: f64) -> u32 {
    (asuint64(d) >> 32) as u32
}

#[inline]
pub(crate) fn get_low_word(d: f64) -> u32 {
    asuint64(d) as u32
}
```

[Visibility]: Internal (不导出) — rusl libm 内部双精度位解析函数

**Intent**: 将一个 `f64` 值的 64-bit 表示拆分为高 32 位(符号 + 指数 + 尾数高位)和低 32 位(尾数低位)。这些函数广泛用于 math 函数的实现中(如 `exp`, `log`, `pow`)。

**前置条件**: `d` 为有效 `f64` 值。

**后置条件**:
- `extract_words(d)` 返回 `(hi, lo)` 其中 `hi` = 高 32 位, `lo` = 低 32 位
- `get_high_word(d)`: 返回高 32 位
- `get_low_word(d)`: 返回低 32 位

**Rust 设计说明**: C 中的 `EXTRACT_WORDS(hi, lo, d)` 宏使用输出参数(out-parameter)模式。Rust 中改为返回值元组，更符合 Rust 惯用法，编译器通常能将元组解构优化为寄存器分配。

---

### 双精度位组合函数

```rust
// Rust — 从高/低 32 位字构造或修改 f64
#[inline]
pub(crate) fn insert_words(hi: u32, lo: u32) -> f64 {
    asdouble(((hi as u64) << 32) | (lo as u64))
}

#[inline]
pub(crate) fn set_high_word(d: f64, hi: u32) -> f64 {
    insert_words(hi, asuint64(d) as u32)
}

#[inline]
pub(crate) fn set_low_word(d: f64, lo: u32) -> f64 {
    insert_words((asuint64(d) >> 32) as u32, lo)
}
```

[Visibility]: Internal (不导出) — rusl libm 内部双精度位构造函数

**Intent**: 从高/低 32 位字构造 `f64` 值，或仅修改高/低字。用于精确控制浮点值的位表示(如修改指数位、符号位)。

**前置条件**: `hi` 和 `lo` 为有效的 32-bit 无符号整数(任意值均可)。

**后置条件**:
- `insert_words(hi, lo)`: 返回由 `(hi << 32) | lo` 位模式构造的 `f64` 值
- `set_high_word(d, hi)`: 返回 `d` 的符号/指数位被 `hi` 替换后的值，低 32 位保持不变
- `set_low_word(d, lo)`: 返回 `d` 的低 32 位被 `lo` 替换后的值，高 32 位保持不变

**Rust 设计说明**: C 中的 `INSERT_WORDS(d, hi, lo)` 宏直接修改左值 `d`。Rust 中改为返回新值(函数式风格)，由调用者赋值。这更安全(无隐蔽的副作用)且编译器能优化掉临时值。

---

### 单精度位操作函数

```rust
// Rust — 从 f32 提取/设置 uint32_t 位模式
#[inline]
pub(crate) fn get_float_word(d: f32) -> u32 {
    asuint(d)
}

#[inline]
pub(crate) fn set_float_word(w: u32) -> f32 {
    asfloat(w)
}
```

[Visibility]: Internal (不导出) — rusl libm 内部单精度位操作函数

**Intent**: `f32` 版本的位提取/设置。`get_float_word` 将 `f32` 解释为 `u32`; `set_float_word` 从 `u32` 构造 `f32`。

---

## 第四部分: 其他宏/常量规约

### WANT_ROUNDING / WANT_SNAN

```rust
// Rust — 功能开关常量
pub(crate) const WANT_ROUNDING: bool = true;
pub(crate) const WANT_SNAN: bool = false;
```

[Visibility]: Internal (不导出) — rusl libm 功能开关

**Intent**:
- `WANT_ROUNDING`: 启用非最近舍入(non-nearest rounding)模式支持。`true` 表示 math 函数需要考虑 `fesetround()` 设置的舍入方向。
- `WANT_SNAN`: 启用 signaling NaN 支持。rusl 设为 `false`(不支持)。

**Rust 设计说明**: C 中使用 `#define WANT_ROUNDING 1`。Rust 中改用 `const bool`，类型安全且可在 `cfg!()` 和条件编译中使用。由于 `WANT_SNAN = false`, SNaN 检测函数恒返回 `false`，编译器可彻底消除相关分支。

---

### issignaling_inline 检测函数

```rust
// Rust — SNaN 检测存根 (因为 WANT_SNAN = false, 恒返回 false)
#[inline]
pub(crate) fn issignalingf_inline(_x: f32) -> bool { false }

#[inline]
pub(crate) fn issignaling_inline(_x: f64) -> bool { false }
```

[Visibility]: Internal (不导出) — rusl libm 的 SNaN 检测存根

**Intent**: 因为 `WANT_SNAN = false`，这两个函数被定义为恒返回 `false`。让编译器彻底消除所有 SNaN 检测分支。

---

## 第五部分: 内部函数规约

### 三角函数归约 (Argument Reduction) 函数

#### rem_pio2 (f64 精度)

```rust
// Rust — 精确 π/2 范围归约
fn rem_pio2(x: f64, y: &mut [f64; 2]) -> c_int;
fn rem_pio2f(x: f32, y: &mut f64) -> c_int;
fn rem_pio2l(x: f128, y: &mut f128) -> c_int;
```

[Visibility]: Internal (不导出) — rusl libm 内部三角函数参数归约

**Intent**: 将输入 `x` 按 π/2 进行精确范围归约，返回 `x` 除以 π/2 的商(整数部分)和余数(小数部分)。这是实现 `sin`、`cos`、`tan` 的基础——三角函数具有 π/2 周期性。

**前置条件**:
- `x`: 有效浮点值(可为任意大小，含 Inf/NaN)
- `y`: 指向可存储结果的缓冲区

**后置条件**:
- 返回值 `n` = `round(x * 2/π)`，即 `x` 中 π/2 的整数倍数
- `y[0]` = `x - n * π/2`(约化到 `[-π/4, π/4]` 的余数)
- 对 `rem_pio2` f64 版本: `y[1]` 存储低位修正项
- `n & 3` 决定使用哪个三角恒等式(sin/cos 符号翻转规则)

**System Algorithm**: 使用 Cody-Waite 归约法: 将 π/2 分解为高精度部分和低精度部分，通过多精度运算获得精确的余数。

**Rust 设计说明**: C 中使用 `double *y` 输出参数。Rust 中 `rem_pio2` 使用 `&mut [f64; 2]` 明确要求 2 个元素的数组引用(编译期长度检查)。`rem_pio2f` 的 `y` 为 `&mut f64`(单元素)。

---

#### rem_pio2_large (f64 高精度)

```rust
// Rust — 大数高精度 π/2 归约
fn rem_pio2_large(x: &mut [f64], y: &mut [f64], e0: c_int, nx: c_int, prec: c_int) -> c_int;
```

[Visibility]: Internal (不导出) — rusl libm 内部大数高精度 π/2 归约

**Intent**: 当 `x` 的绝对值很大(|x| >> π/2)时，标准的 Cody-Waite 归约精度不足。`rem_pio2_large` 使用多精度 π 值表进行高精度归约，适用于 `x` 为以数组表示的任意精度浮点数的情况。

**前置条件**: `x[]` 为包含 `nx` 个 `f64` 的数组，表示高精度浮点数。`e0` 为指数偏移。

**后置条件**: 同 `rem_pio2`，返回商 `n` 并将余数写入 `y[]`。

---

#### __sin / __cos / __tan (f64 精度内核)

```rust
// Rust — 三角函数多乘式近似内核
fn __sin(x: f64, y: f64, n: c_int) -> f64;
fn __cos(x: f64, y: f64) -> f64;
fn __tan(x: f64, y: f64, n: c_int) -> f64;
```

[Visibility]: Internal (不导出) — rusl libm 内部三角函数内核

**Intent**: 这些是三角函数的多项式近似内核，对已经过 π/2 归约的输入计算 sin/cos/tan。

**前置条件**:
- `x`: 归约后的输入值，满足 `|x| <= π/4`
- `y`: 来自 `rem_pio2` 的修正项(低位尾数，提高精度)
- `n`: 象限编号(来自 `rem_pio2` 的返回值)

**后置条件**:
- `__sin(x, y, n)`: 返回 `sin(x + n*π/2)` 的高精度近似值
- `__cos(x, y)`: 返回 `cos(x)` 的高精度近似值
- `__tan(x, y, n)`: 返回 `tan(x + n*π/2)` 的高精度近似值

**System Algorithm**: 使用 Remez 算法生成的多项式逼近，结合双精度算术来提高精度。

---

#### sindf / cosdf / tandf (f32 精度内核)

```rust
// Rust — 单精度三角函数内核 (double 输入, float 输出: "df" 后缀)
fn sindf(x: f64) -> f32;
fn cosdf(x: f64) -> f32;
fn tandf(x: f64, n: c_int) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部单精度三角函数内核

**Intent**: `f32` 版本的三角函数内核。这些函数接受 `f64` 输入(因为归约结果以 `f64` 精度存储)，返回 `f32` 结果。

**注意命名**: `df` 后缀表示 "double input, float output"。Rust 中使用类型签名明确此语义，无需特殊命名约定。

---

#### expo2 / expo2f

```rust
// Rust — 2^x 的尾数构造
fn expo2(x: f64, sign: f64) -> f64;
fn expo2f(x: f32, sign: f32) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部 `2^x` 的尾数构造

**Intent**: 给定一个指数值 `x` 和符号，构造 `sign * 2^x`。这是 `exp` 和 `exp2` 系列函数的底层工具。

**前置条件**: `x` 表示一个缩放后的指数，`sign` 为 ±1.0。

**后置条件**: 返回 `sign * 2^x`。

---

### 多项式求值函数 (long double)

#### polevll / p1evll

```rust
// Rust — long double Horner 多项式求值
fn polevll(x: f128, coef: &[f128], n: c_int) -> f128;
fn p1evll(x: f128, coef: &[f128], n: c_int) -> f128;
```

[Visibility]: Internal (不导出) — rusl libm 内部 long double 多项式求值

**Intent**:
- `polevll`: 使用 Horner 方法计算 `coef[0] + coef[1]*x + coef[2]*x^2 + ... + coef[n]*x^n`
- `p1evll`: 计算 `1 + coef[0]*x + coef[1]*x^2 + ... + coef[n-1]*x^n`(首项系数隐含为 1)

**Rust 设计说明**: C 中使用 `const long double *coef` 裸指针 + `int n` 长度。Rust 中使用 `&[f128]` 切片，自带长度信息且提供边界检查(在非性能关键路径或 debug 模式中)。

---

### lgamma 内部函数

#### signgam (全局变量)

```rust
// Rust — lgamma 符号输出 (内部 + POSIX 公开导出)
pub(crate) static mut __signgam: c_int = 0;

// 对外导出 (C ABI 兼容):
#[no_mangle]
pub static mut signgam: c_int;  // POSIX 标准变量, <math.h> 声明
```

[Visibility]: 复合可见性
- `__signgam`: **Internal** — rusl 内部变量
- `signgam`: **Public** — POSIX 标准变量，`<math.h>` 声明，需以 C ABI 导出

**Intent**: `lgamma()` 函数族通过此全局变量返回 gamma 函数的符号(±1)。因为 `lgamma` 返回的是 log(|Gamma(x)|)，符号通过 `signgam` 补充。

**不变量**: `signgam` 必须为 `-1`, `0`, 或 `1`。每次 `lgamma`/`lgammaf`/`lgammal` 调用后更新。

**ABI 注意**: Rust 中 `signgam` 需使用 `#[no_mangle] pub static mut` 导出为与 C 兼容的全局符号。在 C 实现中 `signgam` 通过 `weak_alias` 绑定到 `__signgam`，Rust 中可直接将 `signgam` 定义为与 `__signgam` 同一地址的符号。

---

#### lgamma_r / lgammaf_r

```rust
// Rust — 可重入版 lgamma (通过输出参数返回符号, 避免修改全局变量)
fn lgamma_r(x: f64, signgamp: &mut c_int) -> f64;
fn lgammaf_r(x: f32, signgamp: &mut c_int) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部，可重入版 lgamma

**Intent**: `lgamma` / `lgammaf` 的内部实现。与 POSIX `lgamma_r` 不同，这些函数接受单独的 `signgamp` 参数以避免修改全局变量 `signgam`。这使得 rusl 可以在多线程环境中安全使用它们。

**Rust 设计说明**: C 中使用 `int *signgamp` 输出参数。Rust 中使用 `&mut c_int` 引用，编译器强制要求可变引用不会同时被多处持有(借用规则保证线程安全)。

---

### 错误处理函数

#### math_invalid / math_invalidf / math_invalidl

```rust
// Rust — 无效操作错误处理 (如 sqrt(-1), 0/0)
fn math_invalid(x: f64) -> f64;
fn math_invalidf(x: f32) -> f32;
// fn math_invalidl(x: f128) -> f128;  // 仅当 LDBL_MANT_DIG != DBL_MANT_DIG
```

[Visibility]: Internal (不导出) — rusl libm 内部数学异常处理(invalid operation)

**Intent**: 处理无效操作(如 `sqrt(-1)`, `0/0`)。根据 math 错误处理模式(`math_errhandling`)执行:
1. 若 `MATH_ERRNO` 设置: `errno = EDOM`
2. 若 `MATH_ERREXCEPT` 设置: 触发 `FE_INVALID` 浮点异常
3. 返回 NaN(静默 NaN，从 `x` 构造)

**前置条件**: `x` 为触发无效操作的值(用于构造 NaN)。

**后置条件**: 返回 `NaN`，已根据当前错误模式设置 `errno` 和/或浮点异常。

---

#### math_oflow / math_oflowf

```rust
// Rust — 浮点上溢处理
fn math_oflow(sign: u32) -> f64;
fn math_oflowf(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部溢出处理

**Intent**: 处理浮点上溢。根据 `sign` 返回 ±Inf 或 ±HUGE_VAL。同时设置 `errno = ERANGE` 和触发 `FE_OVERFLOW` / `FE_INEXACT` 异常。

**前置条件**: `sign` 为 `0`(正) 或 `非0`(负)。

**后置条件**:
- 返回 +∞ (sign==0) 或 -∞ (sign!=0)
- 若 `MATH_ERRNO`: `errno = ERANGE`
- 若 `MATH_ERREXCEPT`: `feraiseexcept(FE_OVERFLOW | FE_INEXACT)`

---

#### math_uflow / math_uflowf

```rust
// Rust — 浮点下溢处理
fn math_uflow(sign: u32) -> f64;
fn math_uflowf(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部下溢处理

**Intent**: 处理浮点下溢。返回带符号的微小值。根据 `math_errhandling` 设置可能触发 `FE_UNDERFLOW`(若结果为非精确值，同时触发 `FE_INEXACT`)。

---

#### math_divzero / math_divzerof

```rust
// Rust — 除零错误处理 (如 log(0) → -∞)
fn math_divzero(sign: u32) -> f64;
fn math_divzerof(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部除零处理

**Intent**: 处理被零除错误(如 `log(0)` → -∞)。返回带符号的无穷大。

---

#### math_xflow / math_xflowf

```rust
// Rust — 上溢/下溢组合处理 (带缩放因子)
fn math_xflow(sign: u32, y: f64) -> f64;
fn math_xflowf(sign: u32, y: f32) -> f32;
```

[Visibility]: Internal (不导出) — rusl libm 内部上溢/下溢(带缩放因子)

**Intent**: 组合上溢/下溢处理与缩放。计算 `sign * y`(`y` 通常是 `0x1p-N` 或 `0x1pN` 缩放因子)，并触发相应的异常。

---

## 全局不变量 (Global Invariants for libm)

1. **精度保证**: 所有内部三角函数内核(`__sin`, `__cos` 等)必须保证结果误差 < 1 ULP(Unit in the Last Place)。

2. **错误处理统一**: 所有 math 公共函数必须通过上述错误处理函数报告错误，不可直接操作 `errno` 或触发异常。

3. **位操作安全**: `asuint` / `asdouble` 等类型双关必须使用 Rust 内建的 `to_bits()` / `from_bits()` 方法，禁止使用 `transmute` 或指针强制转换(前者是安全操作且语义明确)。

4. **float 精度**: `f32` 版本函数(`*f`)应接受 `f64` 精度的中间值作为输入(因为归约步骤需要更高精度)，仅在最终步骤舍入为 `f32`。

5. **NaN 传播**: 若输入为 NaN，所有函数的输出也应为 NaN; 不得将 NaN 错误地转换为普通值或触发不必要的异常。

---

## 实现指南 (rusl/Rust)

- `LdShape` 使用 `#[repr(C)]` union + struct，确保与 C 的布局一致性
- `eval_as_float` / `eval_as_double` 直接使用 Rust 原生 `as` 或赋值(编译器理解这些模式)
- `fp_barrier*` / `fp_force_eval*` 优先使用架构特定 `asm!` 内联汇编；回退使用 `core::hint::black_box`
- 位操作函数使用 `f32::to_bits()` / `f64::to_bits()` / `f32::from_bits()` / `f64::from_bits()` —— 完全安全，无需 unsafe
- `FORCE_EVAL!` 宏可设计为根据 `size_of` 分发，LLVM 能将 `match` 优化为编译期分支消除
- 所有内部数学函数标记为 `pub(crate)`，不对外导出
- `signgam` 的导出: `#[no_mangle] pub static mut signgam: c_int = 0;`，确保 C ABI 兼容
- 错误处理函数使用 `extern "C"` 调用约定(以便被汇编代码或外部 FFI 调用)