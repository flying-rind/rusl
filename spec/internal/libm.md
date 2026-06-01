# libm.h 规约

## 概述

`libm.h` 是 musl 数学库 (`libm`) 的核心内部头文件。它定义了浮点数底层操作所需的类型（`union ldshape`）、内联工具函数（`fp_barrier`、`fp_force_eval`、`eval_as_float` 等）、位操作宏（`EXTRACT_WORDS`、`INSERT_WORDS` 等）、以及大量内部数学辅助函数的声明（三角函数归约、指数函数、错误处理等）。该头文件被 musl `src/math/` 下几乎所有 `.c` 文件包含。

## 依赖图

```
libm.h
├── <stdint.h>     (标准库, uint64_t, uint32_t, uint16_t, int32_t)
├── <float.h>      (标准库, LDBL_MANT_DIG, LDBL_MAX_EXP)
├── <math.h>       (标准库, double_t)
├── <endian.h>     (标准库, __BYTE_ORDER, __LITTLE_ENDIAN, __BIG_ENDIAN)
├── "fp_arch.h"    (架构特定, fp_barrierf/fp_barrier/fp_force_evalf/fp_force_eval)
│
├── [条件编译类型]
│   └── union ldshape    (定义于本文件, 无内部依赖, 四种变体按平台条件选择)
│
├── [内联函数]
│   ├── roundtoint / converttoint   (条件: TOINT_INTRINSICS, 声明但不定义于本文件)
│   ├── eval_as_float / eval_as_double
│   ├── fp_barrierf / fp_barrier / fp_barrierl   (若 fp_arch.h 未提供则定义)
│   └── fp_force_evalf / fp_force_eval / fp_force_evall  (若 fp_arch.h 未提供则定义)
│
├── [宏]
│   ├── predict_true / predict_false
│   ├── FORCE_EVAL
│   ├── asuint / asfloat / asuint64 / asdouble
│   └── EXTRACT_WORDS / GET_HIGH_WORD / GET_LOW_WORD / INSERT_WORDS / SET_HIGH_WORD / SET_LOW_WORD
│       GET_FLOAT_WORD / SET_FLOAT_WORD
│
├── [内部函数 — 三角函数归约 (double)]
│   ├── __rem_pio2_large(double*, double*, int, int, int)  → src/math/__rem_pio2_large.c
│   ├── __rem_pio2(double, double*)                        → src/math/__rem_pio2.c
│   ├── __sin(double, double, int)                          → src/math/__sin.c
│   ├── __cos(double, double)                               → src/math/__cos.c
│   ├── __tan(double, double, int)                          → src/math/__tan.c
│   └── __expo2(double, double)                             → src/math/__expo2.c
│
├── [内部函数 — 三角函数归约 (float)]
│   ├── __rem_pio2f(float, double*)                        → src/math/__rem_pio2f.c
│   ├── __sindf(double)                                     → src/math/__sindf.c
│   ├── __cosdf(double)                                     → src/math/__cosdf.c
│   ├── __tandf(double, int)                                → src/math/__tandf.c
│   └── __expo2f(float, float)                              → src/math/__expo2f.c
│
├── [内部函数 — 三角函数归约 (long double)]
│   ├── __rem_pio2l(long double, long double*)             → src/math/__rem_pio2l.c
│   ├── __sinl(long double, long double, int)               → src/math/__sinl.c
│   ├── __cosl(long double, long double)                    → src/math/__cosl.c
│   └── __tanl(long double, long double, int)               → src/math/__tanl.c
│
├── [内部函数 — 多项式求值 (long double)]
│   ├── __polevll(long double, const long double*, int)    → src/math/__polevll.c
│   └── __p1evll(long double, const long double*, int)     → src/math/__p1evll.c
│
├── [内部函数 — lgamma / signgam]
│   ├── __lgamma_r(double, int*)                           → src/math/__lgamma_r.c
│   └── __lgammaf_r(float, int*)                           → src/math/__lgammaf_r.c
│
├── [全局变量]
│   └── int __signgam                                      → src/math/__signgam.c
│
└── [内部函数 — 错误处理 (math error handling)]
    ├── __math_xflowf(uint32_t, float)                      → src/math/__math_xflowf.c
    ├── __math_uflowf(uint32_t)                             → src/math/__math_uflowf.c
    ├── __math_oflowf(uint32_t)                             → src/math/__math_oflowf.c
    ├── __math_divzerof(uint32_t)                           → src/math/__math_divzerof.c
    ├── __math_invalidf(float)                              → src/math/__math_invalidf.c
    ├── __math_xflow(uint32_t, double)                      → src/math/__math_xflow.c
    ├── __math_uflow(uint32_t)                              → src/math/__math_uflow.c
    ├── __math_oflow(uint32_t)                              → src/math/__math_oflow.c
    ├── __math_divzero(uint32_t)                            → src/math/__math_divzero.c
    ├── __math_invalid(double)                              → src/math/__math_invalid.c
    └── __math_invalidl(long double) [条件: LDBL_MANT_DIG != DBL_MANT_DIG]
```

---

## 第一部分：条件编译类型定义

### union ldshape (四种变体)

`ldshape` 是一个将 `long double` 的原始字节重新解释为各个组成部分（符号/指数/尾数）的联合体。由于 `long double` 在不同平台上有不同的表示（80-bit x87 扩展精度、128-bit IEEE 754 四倍精度），定义了四种变体。

---

#### 变体 1: IEEE 754 double == long double（无 ldshape）

```c
#if LDBL_MANT_DIG == 53 && LDBL_MAX_EXP == 1024
/* ldshape 未定义 — long double 与 double 相同 */
```

[Visibility]: Internal — 条件编译分支

**适用平台**: `long double` 与 `double` 完全相同的架构（如部分 ARM 平台）。在此情况下，所有 `long double` 操作退化为 `double` 操作，不需要 `ldshape`。

---

#### 变体 2: 80-bit x87 扩展精度 — 小端序

```c
#elif LDBL_MANT_DIG == 64 && LDBL_MAX_EXP == 16384 && __BYTE_ORDER == __LITTLE_ENDIAN
union ldshape {
    long double f;
    struct {
        uint64_t m;    /* 64-bit 尾数 */
        uint16_t se;   /* 1-bit 符号 + 15-bit 指数 */
    } i;
};
```
```rust
#[repr(C)]
pub union LdShape {
    pub f: f128, // Rust 无原生 80-bit float, 用 [u8; 10] 或 u128 表示
    pub i: LdShape80LE,
}

#[repr(C)]
pub struct LdShape80LE {
    pub m: u64,    // 尾数 (63 位有效 + 1 位显式整数位)
    pub se: u16,   // 位[15]: 符号, 位[14:0]: 指数 (biased by 16383)
}
```

[Visibility]: Internal (不导出) — musl libm 内部类型，用于在 x87 80-bit 扩展精度平台上操作 long double 的位表示

**适用平台**: x86 (32-bit), x86_64（使用 x87 FPU 时）

**布局不变量**:
- 总大小: 10 字节（80 位），但 `sizeof(union ldshape)` 可能为 12 或 16 字节（对齐填充）
- 内存布局（小端序，低地址→高地址）: `m[0..7]` (8 字节) → `se[0..1]` (2 字节)
- `se` 的位[15] 为符号位
- `se` 的位[14:0] 为偏置指数（bias = 16383）
- `m` 的位[63] 为显式前导位（整数位），正常化为 1；subnormal 时为 0

---

#### 变体 3: 80-bit x87 扩展精度 — 大端序（m68k 变体）

```c
#elif LDBL_MANT_DIG == 64 && LDBL_MAX_EXP == 16384 && __BYTE_ORDER == __BIG_ENDIAN
union ldshape {
    long double f;
    struct {
        uint16_t se;
        uint16_t pad;
        uint64_t m;
    } i;
};
```
```rust
#[repr(C)]
pub struct LdShape80BE {
    pub se: u16,    // 符号 + 指数
    pub pad: u16,   // 对齐填充
    pub m: u64,     // 尾数
}
```

[Visibility]: Internal (不导出) — m68k 大端序 80-bit long double

**适用平台**: Motorola 68000 系列

**限制**: 仅当 `sizeof(uint64_t)` 对齐要求 ≤ 4 字节时才正确。

---

#### 变体 4: IEEE 754 四倍精度 (128-bit) — 小端序

```c
#elif LDBL_MANT_DIG == 113 && LDBL_MAX_EXP == 16384 && __BYTE_ORDER == __LITTLE_ENDIAN
union ldshape {
    long double f;
    struct {
        uint64_t lo;
        uint32_t mid;
        uint16_t top;
        uint16_t se;
    } i;
    struct {
        uint64_t lo;
        uint64_t hi;
    } i2;
};
```
```rust
#[repr(C)]
pub union LdShape {
    pub f: f128,
    pub i: LdShape128LE,
    pub i2: LdShape128LE2,
}

#[repr(C)]
pub struct LdShape128LE {
    pub lo: u64,    // 尾数低 64 位
    pub mid: u32,   // 尾数中 32 位
    pub top: u16,   // 尾数高 16 位
    pub se: u16,    // 1-bit 符号 + 15-bit 指数
}

#[repr(C)]
pub struct LdShape128LE2 {
    pub lo: u64,
    pub hi: u64,    // hi = se << 48 | top << 32 | mid
}
```

[Visibility]: Internal (不导出) — IEEE 754 128-bit 四倍精度

**适用平台**: aarch64 (部分), POWER9+, RISC-V (Q 扩展)

**不变量**:
- `se` 的位[15] = 符号，位[14:0] = 偏置指数 (bias = 16383)
- 尾数 112 位分布: `top`(16) + `mid`(32) + `lo`(64)
- `i2.hi` 将 `se`, `top`, `mid` 打包为一个 `u64`

---

#### 变体 5: IEEE 754 四倍精度 (128-bit) — 大端序

```c
#elif LDBL_MANT_DIG == 113 && LDBL_MAX_EXP == 16384 && __BYTE_ORDER == __BIG_ENDIAN
union ldshape {
    long double f;
    struct {
        uint16_t se;
        uint16_t top;
        uint32_t mid;
        uint64_t lo;
    } i;
    struct {
        uint64_t hi;
        uint64_t lo;
    } i2;
};
```

[Visibility]: Internal (不导出) — 大端序四倍精度

**适用平台**: s390x, 某些 POWER 配置

---

## 第二部分：内联函数规约

### eval_as_float / eval_as_double

```c
static inline float eval_as_float(float x)
{
    float y = x;
    return y;
}

static inline double eval_as_double(double x)
{
    double y = x;
    return y;
}
```
```rust
#[inline]
fn eval_as_float(x: f32) -> f32 {
    let y: f32 = x;
    y
}

#[inline]
fn eval_as_double(x: f64) -> f64 {
    let y: f64 = x;
    y
}
```

[Visibility]: Internal (不导出) — musl libm 内部精度控制工具

**Intent (Level 2)**: 消除 C 语言"额外精度"（excess precision）问题。在 x86 平台上，FPU 寄存器为 80-bit 宽度，`float`/`double` 值在寄存器中可能以扩展精度存在。通过赋值给一个显式类型的局部变量，强制将值舍入到声明类型的精度（`float` → 32-bit, `double` → 64-bit）。这确保计算结果在不同编译器优化级别下保持一致。

**前置条件**: `x` 为有效浮点值。

**后置条件**: 返回 `x` 被舍入到目标类型精度后的值。返回值按 IEEE 754 规则舍入到 `float` (24-bit 尾数) 或 `double` (53-bit 尾数)。

**Invariant**: `eval_as_float(x) == (float)x`, `eval_as_double(x) == (double)x` 按 IEEE 754 语义。

---

### fp_barrierf / fp_barrier / fp_barrierl

```c
#ifndef fp_barrierf
#define fp_barrierf fp_barrierf
static inline float fp_barrierf(float x)
{
    volatile float y = x;
    return y;
}
#endif
```

（`fp_barrier` 和 `fp_barrierl` 同理，分别对应 `double` 和 `long double`）

```rust
// 架构通用回退实现
#[inline]
fn fp_barrierf(x: f32) -> f32 {
    // Rust 中无法直接用 volatile，需要内联汇编或编译器屏障
    // 等效 C: volatile float y = x; return y;
    core::hint::black_box(x)
}
```

[Visibility]: Internal (不导出) — musl libm 内部编译器优化屏障

**Intent (Level 3)**: `fp_barrierf` 充当编译器优化屏障——它强制编译器假设函数调用有副作用并可能返回任意值，从而阻止编译器跨该调用传播关于 `x` 的值的假设。这用于：
1. 防止编译器将浮点常量折叠优化过早应用
2. 分隔浮点异常测试的不同阶段
3. 确保舍入模式更改后生效

`volatile` 强制实际内存读写，阻止编译器优化掉赋值。

**优先级**: 若架构特定的 `fp_arch.h` 提供了内联汇编版本（如 aarch64 使用 `__asm__ __volatile__("" : "+w"(x))`），则覆盖此默认实现。汇编版本更精确，因为它不强制实际内存存储而仅阻止寄存器优化。

**前置条件**: `x` 为有效浮点值（含 NaN/Inf）。

**后置条件**: 返回 `x` 的值，但编译器必须在此处执行对 `x` 的所有待定浮点操作。返回值的位模式与 `x` 相同。

---

### fp_force_evalf / fp_force_eval / fp_force_evall

```c
#ifndef fp_force_evalf
#define fp_force_evalf fp_force_evalf
static inline void fp_force_evalf(float x)
{
    volatile float y;
    y = x;
}
#endif
```

```rust
#[inline]
fn fp_force_evalf(x: f32) {
    core::hint::black_box(x);
}
```

[Visibility]: Internal (不导出) — musl libm 内部浮点表达式求值强制器

**Intent**: `fp_force_evalf` 强制编译器对 `x` 进行完整求值，但**不保留结果**。这用于触发浮点异常（如 underflow、inexact），而不影响程序的计算结果。`volatile` 赋值确保编译器不能消除"无用"的计算。

典型用法：
```c
// 触发 underflow 异常
fp_force_evalf(tiny * tiny);
```

**前置条件**: `x` 为有效浮点值。

**后置条件**: 对 `x` 的求值已完成，可能触发的浮点异常已发生。调用后 `x` 的值被丢弃。无返回值。

---

### FORCE_EVAL 宏

```c
#define FORCE_EVAL(x) do {                        \
    if (sizeof(x) == sizeof(float)) {         \
        fp_force_evalf(x);                \
    } else if (sizeof(x) == sizeof(double)) { \
        fp_force_eval(x);                 \
    } else {                                  \
        fp_force_evall(x);                \
    }                                         \
} while(0)
```

[Visibility]: Internal (不导出) — musl libm 内部类型通用浮点求值强制器

**Intent**: 封装对 `fp_force_evalf` / `fp_force_eval` / `fp_force_evall` 的调用，根据 `x` 的 `sizeof` 自动选择正确的版本。`do { ... } while(0)` 习惯用法确保宏可作为语句安全使用。

**注意**: `sizeof` 的分支在编译期由常量折叠消除（`sizeof()` 是编译期常量），不会产生运行时开销。

---

## 第三部分：位操作宏规约

### 类型双关宏

```c
#define asuint(f)  ((union{float _f; uint32_t _i;}){f})._i
#define asfloat(i) ((union{uint32_t _i; float _f;}){i})._f
#define asuint64(f) ((union{double _f; uint64_t _i;}){f})._i
#define asdouble(i) ((union{uint64_t _i; double _f;}){i})._f
```
```rust
// Rust 等效实现（使用 transmute 或 f32::to_bits / f64::to_bits）
fn asuint(f: f32) -> u32    { f.to_bits() }
fn asfloat(i: u32) -> f32   { f32::from_bits(i) }
fn asuint64(f: f64) -> u64  { f.to_bits() }
fn asdouble(i: u64) -> f64  { f64::from_bits(i) }
```

[Visibility]: Internal (不导出) — musl libm 内部浮点位重新解释宏

**Intent**: 使用 C 的复合字面量 + 匿名联合实现浮点值与整数位模式的零开销转换。这是 C 标准严格别名规则下安全进行类型双关的方法（相比之下，指针强制转换 `*(uint32_t*)&f` 违反严格别名规则）。

**前置条件**:
- `asuint(f)`: `f` 为有效 `float` 值
- `asfloat(i)`: `i` 为任意 32-bit 整数
- `asuint64(f)`: `f` 为有效 `double` 值
- `asdouble(i)`: `i` 为任意 64-bit 整数

**后置条件**: 返回输入值的逐位等价表示。不存在舍入或精度损失。

**不变量**:
- `asfloat(asuint(f)) == f`（往返恒等式）
- `asdouble(asuint64(f)) == f`（往返恒等式）
- 注意：`asuint(asfloat(i))` **不一定等于** `i`（因为存在 NaN 规范化）

---

### 双精度位操作宏

```c
#define EXTRACT_WORDS(hi,lo,d)                    \
do {                                              \
  uint64_t __u = asuint64(d);                     \
  (hi) = __u >> 32;                               \
  (lo) = (uint32_t)__u;                           \
} while (0)

#define GET_HIGH_WORD(hi,d)                       \
do {                                              \
  (hi) = asuint64(d) >> 32;                       \
} while (0)

#define GET_LOW_WORD(lo,d)                        \
do {                                              \
  (lo) = (uint32_t)asuint64(d);                   \
} while (0)
```

[Visibility]: Internal (不导出) — musl libm 内部双精度位解析宏

**Intent**: 将一个 `double` 值的 64-bit 表示拆分为高 32 位（符号 + 指数 + 尾数高位）和低 32 位（尾数低位）。这些宏是对 `asuint64` 的便利封装，广泛用于 musl math 函数的实现中（如 `exp`, `log`, `pow`）。

**前置条件**: `d` 为有效 `double` 值。`hi` 和 `lo` 为可修改的左值（lvalue）。

**后置条件**:
- `EXTRACT_WORDS` 后: `hi` = 高 32 位（符号位 s + 指数 e + 尾数高 20 位 mt），`lo` = 低 32 位（尾数低 32 位 ml）
- `GET_HIGH_WORD` 后: `hi` = 高 32 位
- `GET_LOW_WORD` 后: `lo` = 低 32 位

---

### 双精度位组合宏

```c
#define INSERT_WORDS(d,hi,lo)                     \
do {                                              \
  (d) = asdouble(((uint64_t)(hi)<<32) | (uint32_t)(lo)); \
} while (0)

#define SET_HIGH_WORD(d,hi)                       \
  INSERT_WORDS(d, hi, (uint32_t)asuint64(d))

#define SET_LOW_WORD(d,lo)                        \
  INSERT_WORDS(d, asuint64(d)>>32, lo)
```

[Visibility]: Internal (不导出) — musl libm 内部双精度位构造宏

**Intent**: 从高/低 32 位字构造 `double` 值，或仅修改高/低字。这些宏用于精确控制浮点值的位表示。

**前置条件**: `d` 为可修改左值。`hi` 和 `lo` 为有效的 32-bit 无符号整数。

**后置条件**:
- `INSERT_WORDS` 后: `d` = 由 `(hi << 32) | lo` 位模式构造的 `double` 值
- `SET_HIGH_WORD` 后: `d` 的符号/指数位被 `hi` 替换，低 32 位保持不变
- `SET_LOW_WORD` 后: `d` 的低 32 位被 `lo` 替换，高 32 位保持不变

---

### 单精度位操作宏

```c
#define GET_FLOAT_WORD(w,d)                       \
do {                                              \
  (w) = asuint(d);                                \
} while (0)

#define SET_FLOAT_WORD(d,w)                       \
do {                                              \
  (d) = asfloat(w);                               \
} while (0)
```

[Visibility]: Internal (不导出) — musl libm 内部单精度位操作宏

**Intent**: `float` 版本的位提取/设置宏。`GET_FLOAT_WORD` 将 `float` 解释为 `uint32_t`；`SET_FLOAT_WORD` 从 `uint32_t` 构造 `float`。

---

## 第四部分：其他宏规约

### WANT_ROUNDING / WANT_SNAN

```c
#define WANT_ROUNDING 1
#define WANT_SNAN 0
```

[Visibility]: Internal (不导出) — musl libm 功能开关

**Intent**:
- `WANT_ROUNDING`: 启用非最近舍入（non-nearest rounding）模式支持。设为 `1` 意味着 math 函数需要考虑 `fesetround()` 设置的舍入方向。
- `WANT_SNAN`: 启用 signaling NaN 支持。musl 设为 `0`（不支持），若设为 `1` 则会触发 `#error`。

---

### issignalingf_inline / issignaling_inline

```c
#define issignalingf_inline(x) 0
#define issignaling_inline(x) 0
```

[Visibility]: Internal (不导出) — musl libm 的 SNaN 检测存根

**Intent**: 因为 `WANT_SNAN = 0`，这两个宏被定义为常量 `0`（永不触发）。让编译器彻底消除所有 SNaN 检测分支。

---

### predict_true / predict_false

```c
#ifdef __GNUC__
#define predict_true(x)  __builtin_expect(!!(x), 1)
#define predict_false(x) __builtin_expect(x, 0)
#else
#define predict_true(x)  (x)
#define predict_false(x) (x)
#endif
```

[Visibility]: Internal (不导出) — 分支预测提示宏

**Intent**: 向编译器提供分支预测信息，优化热路径代码布局。
- `predict_true(x)`: 提示 `x` 很可能为真
- `predict_false(x)`: 提示 `x` 很可能为假

在 `__GNUC__` 下使用 `__builtin_expect`；在其他编译器下退化为恒等宏。

---

## 第五部分：内部函数规约

### 三角函数归约 (Argument Reduction) 函数

#### __rem_pio2 (double 精度)

```c
hidden int __rem_pio2(double x, double *y);
hidden int __rem_pio2f(float x, double *y);
hidden int __rem_pio2l(long double x, long double *y);
```
```rust
fn __rem_pio2(x: f64, y: *mut f64) -> c_int;
fn __rem_pio2f(x: f32, y: *mut f64) -> c_int;
fn __rem_pio2l(x: f128, y: *mut f128) -> c_int;
```

[Visibility]: Internal (不导出) — musl libm 内部三角函数参数归约

**Intent**: 将输入 `x` 按 π/2 进行精确范围归约，返回 `x` 除以 π/2 的商（整数部分）和余数（小数部分）。这是实现 `sin`、`cos`、`tan` 的基础——三角函数具有 π/2 周期性。

**前置条件**:
- `x`: 有效浮点值（可为任意大小，含 Inf/NaN）
- `y`: 指向可存储结果的缓冲区

**后置条件**:
- 返回值 `n` = `round(x * 2/π)`，即 `x` 中 π/2 的整数倍数
- `y[0]` = `x - n * π/2`（约化到 `[-π/4, π/4]` 的余数），对 `__rem_pio2` 双精度版本，`y[1]` 存储低位修正项
- `n & 3` 决定使用哪个三角恒等式（sin/cos 符号翻转规则）

**System Algorithm**: 使用 Cody-Waite 归约法：将 π/2 分解为高精度部分和低精度部分，通过多精度运算获得精确的余数。

---

#### __rem_pio2_large (double 高精度)

```c
hidden int __rem_pio2_large(double *x, double *y, int e0, int nx, int prec);
```
```rust
fn __rem_pio2_large(x: *mut f64, y: *mut f64, e0: c_int, nx: c_int, prec: c_int) -> c_int;
```

[Visibility]: Internal (不导出) — musl libm 内部大数高精度 π/2 归约

**Intent**: 当 `x` 的绝对值很大（|x| >> π/2）时，标准的 Cody-Waite 归约精度不足。`__rem_pio2_large` 使用多精度 π 值表进行高精度归约，适用于 `x` 为以数组表示的任意精度浮点数的情况。

**前置条件**: `x[]` 为包含 `nx` 个 `double` 的数组，表示高精度浮点数。`e0` 为指数偏移。

**后置条件**: 同 `__rem_pio2`，返回商 `n` 并将余数写入 `y[]`。

---

#### __sin / __cos / __tan (double 精度内核)

```c
hidden double __sin(double x, double y, int n);
hidden double __cos(double x, double y);
hidden double __tan(double x, double y, int n);
```
```rust
fn __sin(x: f64, y: f64, n: c_int) -> f64;
fn __cos(x: f64, y: f64) -> f64;
fn __tan(x: f64, y: f64, n: c_int) -> f64;
```

[Visibility]: Internal (不导出) — musl libm 内部三角函数内核

**Intent**: 这些是三角函数的多项式近似内核，对已经过 π/2 归约的输入计算 sin/cos/tan。

**前置条件**:
- `x`: 归约后的输入值，满足 `|x| <= π/4`
- `y`: 来自 `__rem_pio2` 的修正项（低位尾数，提高精度）
- `n`: 象限编号（来自 `__rem_pio2` 的返回值）

**后置条件**:
- `__sin(x, y, n)`: 返回 `sin(x + n*π/2)` 的高精度近似值
- `__cos(x, y)`: 返回 `cos(x)` 的高精度近似值
- `__tan(x, y, n)`: 返回 `tan(x + n*π/2)` 的高精度近似值

**System Algorithm**: 使用 Remez 算法生成的多项式逼近，结合双精度算术来提高精度。

---

#### __sindf / __cosdf / __tandf (float 精度内核)

```c
hidden float __sindf(double x);
hidden float __cosdf(double x);
hidden float __tandf(double x, int n);
```
```rust
fn __sindf(x: f64) -> f32;
fn __cosdf(x: f64) -> f32;
fn __tandf(x: f64, n: c_int) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部单精度三角函数内核

**Intent**: `float` 版本的三角函数内核。这些函数接受 `double` 输入（因为归约结果以 `double` 精度存储），返回 `float` 结果。

**注意命名**: `df` 后缀表示 "double input, float output"。

---

#### __expo2 / __expo2f

```c
hidden double __expo2(double x, double sign);
hidden float __expo2f(float x, float sign);
```
```rust
fn __expo2(x: f64, sign: f64) -> f64;
fn __expo2f(x: f32, sign: f32) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部 `2^x` 的尾数构造

**Intent**: 给定一个指数值 `x` 和符号，构造 `sign * 2^x`。这是 `exp` 和 `exp2` 系列函数的底层工具。

**前置条件**: `x` 表示一个缩放后的指数，`sign` 为 ±1.0。

**后置条件**: 返回 `sign * 2^x`。

---

### 多项式求值函数 (long double)

#### __polevll / __p1evll

```c
hidden long double __polevll(long double x, const long double *coef, int n);
hidden long double __p1evll(long double x, const long double *coef, int n);
```
```rust
fn __polevll(x: f128, coef: *const f128, n: c_int) -> f128;
fn __p1evll(x: f128, coef: *const f128, n: c_int) -> f128;
```

[Visibility]: Internal (不导出) — musl libm 内部 long double 多项式求值

**Intent**:
- `__polevll`: 使用 Horner 方法计算 `coef[0] + coef[1]*x + coef[2]*x^2 + ... + coef[n]*x^n`
- `__p1evll`: 计算 `1 + coef[0]*x + coef[1]*x^2 + ... + coef[n-1]*x^n`（首项系数隐含为 1）

---

### lgamma 内部函数

#### __signgam (全局变量)

```c
extern int __signgam;
```

[Visibility]: 复合可见性
- `__signgam`: **Internal** — musl 内部变量
- `signgam`: **Public** — POSIX 标准变量，`<math.h>` 声明，通过 `weak_alias` 暴露

**Intent**: `lgamma()` 函数族通过此全局变量返回 gamma 函数的符号（±1）。因为 `lgamma` 返回的是 log(|Gamma(x)|)，符号通过 `signgam` 补充。

**不变量**: `__signgam` 必须为 `-1`, `0`, 或 `1`。每次 `lgamma`/`lgammaf`/`lgammal` 调用后更新。

---

#### __lgamma_r / __lgammaf_r

```c
hidden double __lgamma_r(double x, int *signgamp);
hidden float __lgammaf_r(float x, int *signgamp);
```
```rust
fn __lgamma_r(x: f64, signgamp: *mut c_int) -> f64;
fn __lgammaf_r(x: f32, signgamp: *mut c_int) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部，可重入版 lgamma

**Intent**: `lgamma` / `lgammaf` 的内部实现。与 POSIX `lgamma_r` 不同，这些函数接受单独的 `signgamp` 参数以避免修改全局变量 `__signgam`。这使得 musl 可以在多线程环境中安全使用它们。

---

### 错误处理函数

#### __math_invalid / __math_invalidf / __math_invalidl

```c
hidden double __math_invalid(double x);
hidden float __math_invalidf(float x);
hidden long double __math_invalidl(long double x);  /* 条件编译 */
```
```rust
fn __math_invalid(x: f64) -> f64;
fn __math_invalidf(x: f32) -> f32;
// fn __math_invalidl(x: f128) -> f128;  // 仅当 LDBL_MANT_DIG != DBL_MANT_DIG
```

[Visibility]: Internal (不导出) — musl libm 内部数学异常处理（invalid operation）

**Intent**: 处理无效操作（如 `sqrt(-1)`, `0/0`）。根据 math 错误处理模式（`math_errhandling`）执行：
1. 若 `MATH_ERRNO` 设置：`errno = EDOM`
2. 若 `MATH_ERREXCEPT` 设置：触发 `FE_INVALID` 浮点异常
3. 返回 NaN（静默 NaN，从 `x` 构造）

**前置条件**: `x` 为触发无效操作的值（用于构造 NaN）。

**后置条件**: 返回 `NaN`，已根据当前错误模式设置 `errno` 和/或浮点异常。

---

#### __math_oflow / __math_oflowf

```c
hidden double __math_oflow(uint32_t sign);
hidden float __math_oflowf(uint32_t sign);
```
```rust
fn __math_oflow(sign: u32) -> f64;
fn __math_oflowf(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部溢出处理

**Intent**: 处理浮点上溢。根据 `sign` 返回 ±Inf 或 ±HUGE_VAL。同时设置 `errno = ERANGE` 和触发 `FE_OVERFLOW` / `FE_INEXACT` 异常。

**前置条件**: `sign` 为 `0`（正）或 `非0`（负）。

**后置条件**:
- 返回 +∞（sign==0）或 -∞（sign!=0）
- 若 `MATH_ERRNO`: `errno = ERANGE`
- 若 `MATH_ERREXCEPT`: `feraiseexcept(FE_OVERFLOW | FE_INEXACT)`

---

#### __math_uflow / __math_uflowf

```c
hidden double __math_uflow(uint32_t sign);
hidden float __math_uflowf(uint32_t sign);
```
```rust
fn __math_uflow(sign: u32) -> f64;
fn __math_uflowf(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部下溢处理

**Intent**: 处理浮点下溢。返回带符号的微小值。根据 `math_errhandling` 设置可能触发 `FE_UNDERFLOW`（若结果为非精确值，同时触发 `FE_INEXACT`）。

---

#### __math_divzero / __math_divzerof

```c
hidden double __math_divzero(uint32_t sign);
hidden float __math_divzerof(uint32_t sign);
```
```rust
fn __math_divzero(sign: u32) -> f64;
fn __math_divzerof(sign: u32) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部除零处理

**Intent**: 处理被零除错误（如 `log(0)` → -∞）。返回带符号的无穷大。

---

#### __math_xflow / __math_xflowf

```c
hidden double __math_xflow(uint32_t sign, double y);
hidden float __math_xflowf(uint32_t sign, float y);
```
```rust
fn __math_xflow(sign: u32, y: f64) -> f64;
fn __math_xflowf(sign: u32, y: f32) -> f32;
```

[Visibility]: Internal (不导出) — musl libm 内部上溢/下溢（带缩放因子）

**Intent**: 组合上溢/下溢处理与缩放。计算 `sign * y`（`y` 通常是 `0x1p-N` 或 `0x1pN` 缩放因子），并触发相应的异常。

---

## 全局不变量 (Global Invariants for libm)

1. **精度保证**: 所有内部三角函数内核 (`__sin`, `__cos` 等) 必须保证结果误差 < 1 ULP（Unit in the Last Place）。

2. **错误处理统一**: 所有 math 公共函数必须通过上述错误处理函数报告错误，不可直接操作 `errno` 或触发异常。

3. **位操作安全**: `asuint` / `asdouble` 等类型双关必须使用 union（C）或 `to_bits` / `from_bits`（Rust），禁止指针强制转换。

4. **float 精度**: `float` 版本函数 (`*f`) 应接受 `double` 精度的中间值作为输入（因为归约步骤需要更高精度），仅在最终步骤舍入为 `float`。

5. **NaN 传播**: 若输入为 NaN，所有函数的输出也应为 NaN；不得将 NaN 错误地转换为普通值或触发不必要的异常。