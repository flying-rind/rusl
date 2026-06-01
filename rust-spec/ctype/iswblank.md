# iswblank 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 类型定义

```rust
// wint_t: 对应 C 的 typedef unsigned wint_t (来自 <bits/alltypes.h>)
// 在 musl 中 wint_t 即 unsigned int, Rust 中映射为 core::ffi::c_uint (u32)
pub type wint_t = core::ffi::c_uint;

// locale_t: 对应 C 的 struct __locale_struct * (来自 <bits/alltypes.h>)
// 不透明结构体指针, 具体定义见 src/internal/locale_impl.h
#[repr(C)]
pub struct __locale_struct { _opaque: [u8; 1] }
pub type locale_t = *mut __locale_struct;

// WEOF: 对应 C 的 #define WEOF 0xffffffffU (来自 <wctype.h>)
pub const WEOF: wint_t = 0xffffffff_u32;
```

---

## 函数接口

```rust
use core::ffi::c_int;

/// C 标准 iswblank — 判断宽字符是否为空白字符（空格或水平制表符）
/// [Visibility]: External — POSIX.1-2001 标准函数, <wctype.h> 声明, ABI 兼容
pub extern "C" fn iswblank(wc: wint_t) -> c_int;

/// iswblank_l_impl — iswblank_l 的内部实现体 (C 中对应 __iswblank_l)
/// [Visibility]: Internal — musl 内部符号, 不直接对外导出
/// Rust 无 weak_alias 机制, 更名为 iswblank_l_impl 并作为模块内部函数
pub(crate) fn iswblank_l_impl(wc: wint_t, l: locale_t) -> c_int;

/// iswblank_l — locale-aware 宽字符空白判断
/// [Visibility]: External — POSIX.1-2008 标准函数, extern "C" 导出, ABI 兼容
/// 注: musl 不区分 locale, l 参数被忽略, 行为与 iswblank 完全一致
pub extern "C" fn iswblank_l(wc: wint_t, l: locale_t) -> c_int;
```

### 符号可见性

| 符号 | 可见性 | 导出方式 | 说明 |
|---|---|---|---|
| `iswblank` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2001 标准, `<wctype.h>` 声明 |
| `iswblank_l` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2008 标准, `<wctype.h>` 声明 |
| `iswblank_l_impl` | Internal | `pub(crate) fn` (模块私有) | 共享实现体, C 中对应 `__iswblank_l` |

注: Rust 无 `weak_alias` 机制，将 `__iswblank_l` 更名为 `iswblank_l_impl` 并作为模块内部函数。`iswblank_l` 直接定义为独立的 `extern "C"` 函数，内部委托给 `iswblank_l_impl`。对外部调用者而言，`iswblank_l` 的行为与 C 的 `weak_alias(__iswblank_l, iswblank_l)` 等价。

### 前置/后置条件

**[Pre-condition]:**
- `wc`: 类型为 `wint_t` (即 `c_uint`)，任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。

**[Post-condition]:**
- Case 1: `wc` 是空白字符（空格 `L' ' = 0x20` 或水平制表符 `L'\t' = 0x09`）
  - 返回非零值（委托给 `isblank(wc as c_int)`）。
- Case 2: 其他字符或 `WEOF`
  - 返回 `0`。

**[Post-condition] (iswblank_l / iswblank_l_impl):**
- 完全等效于 `iswblank(wc)` 的返回值。
- `l` 参数（`*mut __locale_struct`）被忽略，所有 locale 下行为一致。

### 不变量

**[Invariant]:**
- 纯函数，无副作用，无内部可变状态，完全线程安全。
- 直接委托给 `isblank`。由于 ASCII 空格 (`0x20`) 和水平制表符 (`0x09`) 在宽字符编码中的值与 `char` 完全相同，`iswblank(wc)` 可安全地将 `wc` 向下转型为 `c_int` 并调用 `isblank`。

### 意图

判断宽字符是否为空白字符（空格或水平制表符）。与 `isspace` 不同，`iswblank` 仅识别空格和水平制表符（POSIX "blank" 字符类），不包含换行、垂直制表符等其他空白字符。由于目标字符均在 ASCII 范围内且宽字符编码与 ASCII 同值，musl 直接委托给 `isblank`，避免重复实现。Rust 中继续保持此委托模式。

### 系统算法

```rust
fn iswblank(wc: wint_t) -> c_int {
    // 空格和水平制表符的 wint_t 值与对应 char 值完全相同:
    //   L' ' = 0x20, L'\t' = 0x09
    // 因此可安全地向下转型为 c_int 并委托给 isblank
    //
    // 注: wint_t 为 c_uint (u32), c_int 为 i32
    //     wc < 0x80000000 时转型安全 (不会符号位翻转)
    //     对于 WEOF = 0xffffffff, 转型为 i32 后为 -1 (即 EOF)
    //     isblank 正确处理 EOF → 返回 0
    isblank(wc as c_int)
}

fn iswblank_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswblank(wc)  // 或直接调用 isblank(wc as c_int)
}
```

时间复杂度 O(1)。`isblank` 内部实现为 `(c as c_uint) == ' ' as c_uint || (c as c_uint) == '\t' as c_uint`，共 2 次比较。

---

## 跨文件依赖摘要

| 依赖符号 | 来源文件 | 可见性 | 说明 |
|---|---|---|---|
| `isblank` | `src/ctype/isblank.rs` | External | 见 `src/ctype/rust-spec/isblank.md`, 判断字符是否为空格或水平制表符 |
| `wint_t` | 本模块定义 (`c_uint` 别名) | 系统类型 | `c_uint` 的 pub type 别名 |
| `locale_t` | 本模块引用 (`*mut __locale_struct`) | 系统类型 | `*mut __locale_struct` 的 pub type 别名 |
| `WEOF` | 本模块定义 | 模块常量 | 值为 `0xffffffff_u32` |

---

## 依赖汇总

```
[RELY]
Predefined Types:
  c_int                     // core::ffi::c_int, 对应 C 的 int (函数返回类型和参数转换目标)
  c_uint                    // core::ffi::c_uint, 对应 C 的 unsigned int
  wint_t                    // pub type wint_t = c_uint;
  __locale_struct           // 不透明结构体, 见 src/internal/locale_impl.h
                            // Rust: #[repr(C)] struct __locale_struct { _opaque: [u8; 1] }
  locale_t                  // pub type locale_t = *mut __locale_struct;
  WEOF                      // pub const WEOF: wint_t = 0xffffffff_u32;

Predefined Functions (External, extern "C" 导出):
  isblank(c: c_int) -> c_int     // 外部依赖: src/ctype/isblank.rs
                                 // 判断字符是否为空格 (' ') 或水平制表符 ('\t')
                                 // iswblank 将 wc 向下转型为 c_int 后委托给 isblank

[GUARANTEE]
Exported Interface (extern "C", ABI 兼容):
  pub extern "C" fn iswblank(wc: wint_t) -> c_int;
                              // 本模块保证对外提供的 iswblank 接口签名与行为
  pub extern "C" fn iswblank_l(wc: wint_t, l: locale_t) -> c_int;
                              // 本模块保证对外提供的 iswblank_l 接口签名与行为
                              // 注: l 参数被忽略, 行为等效于 iswblank(wc)

Internal Interface (不对外导出):
  pub(crate) fn iswblank_l_impl(wc: wint_t, l: locale_t) -> c_int;
                              // 内部实现函数, 作为 iswblank_l 的委托目标
                              // C 中对应 __iswblank_l, Rust 中重命名为 iswblank_l_impl
```