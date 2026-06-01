# complex_impl.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/complex_impl.h`
> **复杂度等级**: Level 1（简单模块 — 宏定义 + 函数声明，前置/后置条件即可描述）

---

## 依赖图

```
(外部) <complex.h> ──> complex_impl.h
(外部) libm.h ──────> complex_impl.h
                           │
                           ├── __CMPLX 宏（C11 复合字面量构造复数）
                           ├── CMPLX / CMPLXF / CMPLXL（类型特化宏）
                           ├── __ldexp_cexp (hidden 函数声明)
                           └── __ldexp_cexpf (hidden 函数声明)
```

本文件为 musl 数学库的复数运算模块提供基础设施：复数构造宏和两个内部复数函数声明。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<complex.h>` | C 标准库 | 跳过 — 提供 `_Complex` 关键字和标准复数类型 |
| `libm.h` | musl 内部头文件 | **跨文件内部依赖** — 提供 `hidden` 可见性宏和数学库浮点基础设施（见 libm.h spec） |

---

## 符号规约

---

### `__CMPLX`

```c
#define __CMPLX(x, y, t) \
    ((union { _Complex t __z; t __xy[2]; }){.__xy = {(x),(y)}}.__z)
```

[Visibility]: Internal — musl 数学库内部宏，POSIX/C 标准未定义。用于内部实现，不通过公共头文件暴露。

#### 功能意图 (Intent)

通过 C11 复合字面量（compound literal）和匿名联合构造一个复数。利用 C 标准保证 `_Complex T` 的内存布局与 `T[2]` 相同的特性（实部在前，虚部在后），将两个标量 `(x, y)` 打包为一个复数类型 `_Complex t`。

#### 系统算法 (System Algorithm)

```
1. 创建匿名联合类型: union { _Complex t __z; t __xy[2]; }
2. 通过复合字面量初始化 .__xy 为 {x, y}
3. 读取 .__z 成员，获得复数形式的值
```

类型别名替换（type punning）通过联合而非指针转换实现，符合 C 标准严格别名规则。

#### 前置条件 (Preconditions)

- **PRE-1**: `t` 必须是一个合法的复数基类型（`float`、`double` 或 `long double`）。
- **PRE-2**: `x` 和 `y` 的类型必须能隐式转换为类型 `t`。
- **PRE-3**: 编译器必须支持 C11 复合字面量语法（`(type){...}`）。

#### 后置条件 (Postconditions)

- **POST-1**: 宏展开为一个类型为 `_Complex t` 的值，其实部为 `(t)(x)`，虚部为 `(t)(y)`。
- **POST-2**: 构造过程不产生函数调用，完全在编译期完成（或内联为寄存器操作）。

#### 不变量 (Invariants)

- **INV-1**: 对于任意类型 `t`，`__CMPLX(x, 0, t)` 的虚部必须为精确的 +0.0。
- **INV-2**: 该宏依赖于 `_Complex T` 与 `T[2]` 内存布局兼容的 C 标准保证（ISO C11 6.2.5p13）。

---

### `CMPLX`

```c
#define CMPLX(x, y) __CMPLX(x, y, double)
```

[Visibility]: Internal — musl 内部宏，等价于 C11 标准 `CMPLX` 宏的非标准前身。

#### 后置条件 (Postconditions)

- **POST-1**: 返回类型为 `double complex` 的复数，实部 = `(double)(x)`，虚部 = `(double)(y)`。

---

### `CMPLXF`

```c
#define CMPLXF(x, y) __CMPLX(x, y, float)
```

[Visibility]: Internal — musl 内部宏。

#### 后置条件 (Postconditions)

- **POST-1**: 返回类型为 `float complex` 的复数。

---

### `CMPLXL`

```c
#define CMPLXL(x, y) __CMPLX(x, y, long double)
```

[Visibility]: Internal — musl 内部宏。

#### 后置条件 (Postconditions)

- **POST-1**: 返回类型为 `long double complex` 的复数。

---

### `__ldexp_cexp`

```c
hidden double complex __ldexp_cexp(double complex, int);
```

[Visibility]: Internal — musl 数学库内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

计算 `ldexp(cexp(z), n)` 的组合操作，即 `exp(z) * 2^n`。通过**融合乘幂操作**避免了先计算 `exp(z)` 再乘 `2^n` 可能导致的中间溢出/下溢问题——当 `exp(z)` 超出 double 表示范围但 `exp(z) * 2^n` 在范围内时，单独计算会导致错误。

#### 前置条件 (Preconditions)

- **PRE-1**: 第一个参数为任意有效的 `double complex` 值（包括 NaN、Inf、次正规数）。
- **PRE-2**: 第二个参数 `n` 为整数指数。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `exp(z) * 2^n`（在 `double complex` 精度范围内）。
- **POST-2**: 若结果在 double 范围内可表示，即使中间结果 `exp(z)` 单独不可表示，函数也能正确返回（无虚假溢出/下溢）。

#### 不变量 (Invariants)

- **INV-1**: 对于任意 `z` 和 `n`，`__ldexp_cexp(z, n) == ldexp(cexp(z), n)` 在数学上成立（但前者在边界情况下更精确）。
- **INV-2**: 该函数是 `cexp` 实现的关键内部组件，不应被外部代码直接调用。

---

### `__ldexp_cexpf`

```c
hidden float complex __ldexp_cexpf(float complex, int);
```

[Visibility]: Internal — musl 数学库内部函数，POSIX/C 标准未定义。

#### 功能意图 (Intent)

`__ldexp_cexp` 的 `float` 版本。计算 `ldexpf(cexpf(z), n)` 的融合操作。

#### 前置条件 (Preconditions)

- **PRE-1**: 第一个参数为任意有效的 `float complex` 值。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `exp(z) * 2^n`（在 `float complex` 精度范围内）。
- **POST-2**: 避免中间溢出/下溢。

---

## 全局不变量

- **GINV-1**: `complex_impl.h` 被包含前，调用方必须先 `#undef __CMPLX`、`#undef CMPLX` 等（本文件内部已做），确保不会与标准 `<complex.h>` 中可能定义的 `CMPLX` 宏冲突。
- **GINV-2**: 所有 `complex_impl.h` 中的 `hidden` 函数仅对 libc.so 内部可见，外部程序无法链接到这些符号。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `hidden` 宏 | `features.h`（见 features.h spec） | 可见性控制 |
| `__ldexp_cexp` 实现 | `src/complex/__ldexp_cexp.c` | 实现位于独立 .c 文件 |
| `__ldexp_cexpf` 实现 | `src/complex/__ldexp_cexpf.c` | 实现位于独立 .c 文件 |
| `cexp` (公共API) | `src/complex/cexp.c` | `cexp` 内部调用 `__ldexp_cexp` |

---

## Rust 实现提示 (`#![no_std]`)

在 `rusl` 中，`CMPLX`/`CMPLXF`/`CMPLXL` 宏应使用 Rust 的内建复数语法替代：

| C 宏 | Rust 等价 |
|------|-----------|
| `CMPLX(x, y)` | `Complex64::new(x, y)` (来自 `num-complex` 或自定义 `Complex` 类型) |
| `CMPLXF(x, y)` | `Complex32::new(x, y)` |
| `CMPLXL(x, y)` | 需自定义 `Complex128` 类型（Rust 原生不支持 `long double`） |

`__ldexp_cexp` / `__ldexp_cexpf`:
- 在 `rusl` 中作为内部函数实现（`pub(crate)` 可见性）。
- 使用 `libm` crate 或自行实现的 `exp`/`ldexp` 融合。