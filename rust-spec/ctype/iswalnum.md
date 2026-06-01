# iswalnum 函数规约 (Rust)

## 复杂度分级: Level 1

---

## 依赖图

```
iswalnum ──→ iswdigit (外部, see src/ctype/iswdigit.rs)
         └─→ iswalpha (外部, see src/ctype/iswalpha.rs)
iswalnum_l_impl ──→ iswalnum (本模块)
iswalnum_l ──→ iswalnum_l_impl (本模块, 内部委托)
```

注: Rust 无 weak_alias 机制。C 中的 `__iswalnum_l` 更名为 `iswalnum_l_impl`（模块内部函数），`iswalnum_l` 为独立的 `extern "C"` 导出函数，内部委托给 `iswalnum_l_impl`。

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

/// C 标准 iswalnum — 判断宽字符是否为字母或数字
/// [Visibility]: External — POSIX.1-2001 标准函数, <wctype.h> 声明, ABI 兼容
pub extern "C" fn iswalnum(wc: wint_t) -> c_int;

/// iswalnum_l_impl — iswalnum_l 的内部实现体 (C 中对应 __iswalnum_l)
/// [Visibility]: Internal — musl 内部符号, 不直接对外导出
/// Rust 无 weak_alias 机制, 更名为 iswalnum_l_impl 并作为模块内部函数
pub(crate) fn iswalnum_l_impl(wc: wint_t, l: locale_t) -> c_int;

/// iswalnum_l — locale-aware 宽字符字母/数字判断
/// [Visibility]: External — POSIX.1-2008 标准函数, extern "C" 导出, ABI 兼容
/// 注: musl 不区分 locale, l 参数被忽略, 行为与 iswalnum 完全一致
pub extern "C" fn iswalnum_l(wc: wint_t, l: locale_t) -> c_int;
```

### 符号可见性

| 符号 | 可见性 | 导出方式 | 说明 |
|---|---|---|---|
| `iswalnum` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2001 标准, `<wctype.h>` 声明 |
| `iswalnum_l` | External | `#[no_mangle] pub extern "C"` | POSIX.1-2008 标准, `<wctype.h>` 声明 |
| `iswalnum_l_impl` | Internal | `pub(crate) fn` (模块私有) | 共享实现体, C 中对应 `__iswalnum_l` |

## iswalnum (对外导出)

**[Visibility]: External** — POSIX.1-2001 标准函数，`<wctype.h>` 声明。

### 前置/后置条件

**[Pre-condition]:**
- `wc`: 类型为 `wint_t` (即 `c_uint`)，任意宽字符值（含 `WEOF` = `0xffffffff_u32`）。

**[Post-condition]:**
- Case 1 — `wc` 是宽字符十进制数字或宽字符字母:
  - `iswdigit(wc)` 为 `true` 时，返回 `1`（数字分支优先，返回值固定为 `1`）。
  - `iswdigit(wc)` 为 `false` 但 `iswalpha(wc)` 为 `true` 时，返回非零值（musl 中为 `1`）。
- Case 2 — `wc` 既不是数字也不是字母，或 `wc == WEOF`:
  - 返回 `0`。

### 不变量

**[Invariant]:**
- 纯函数，无副作用，无内部状态，完全线程安全。
- 本实现不依赖 locale 设置（locale 参数仅存在于 `_l` 后缀版本中，且被忽略）。

### 意图

判断宽字符是否属于字母或数字（POSIX "alnum" 字符类）。musl 实现采用"数字优先检测"策略：先执行代价极低的 `iswdigit` 快速路径检查（单次无符号范围比较），仅在该检查失败后才调用 `iswalpha` 进行位图查表——因为大多数输入文本中字母比数字更常见，但数字检查更快（无分支、无内存访问）。在 Rust 中可利用 `#[inline]` + 布尔短路求值进一步优化。

### 系统算法

```rust
fn iswalnum(wc: wint_t) -> c_int {
    // Step 1 — 数字快速路径:
    //   iswdigit(wc): (wc).wrapping_sub('0' as wint_t) < 10
    //   若成立 → 返回 1
    //   时间复杂度 O(1), 无分支（可 cmov 实现）, 无内存访问
    //
    // Step 2 — 字母查表路径（仅在 Step 1 为 false 时执行）:
    //   iswalpha(wc): 二级位图查表 (见 src/ctype/rust-spec/iswalpha.md)
    //   返回 iswalpha 的结果（musl 中命中时为 1, 未命中时为 0）
    //
    // Rust 惯用写法:
    if (wc.wrapping_sub('0' as wint_t) < 10) || (iswalpha(wc) != 0) { 1 } else { 0 }
    // 或利用布尔短路:
    //   (wc.wrapping_sub('0' as wint_t) < 10) as c_int | iswalpha(wc)
}

fn iswalnum_l_impl(wc: wint_t, _l: locale_t) -> c_int {
    iswalnum(wc)
}
```

总体时间复杂度 O(1)，最坏情况仅 1 次无符号比较 + 1 次位图查表。

---

## iswalnum_l_impl (内部符号)

**[Visibility]: Internal** — musl 内部辅助函数（C 中 `__` 前缀），POSIX/C 标准未定义。在 Rust 中更名为 `iswalnum_l_impl`，作为 `iswalnum_l` 的底层实现存在。

### 前置/后置条件

**[Pre-condition]:**
- `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
- `l`: 类型为 `locale_t`（`*mut __locale_struct`），指向有效的 locale 结构或为 `null_mut()`（表示 C locale）。

**[Post-condition]:**
- 完全等效于 `iswalnum(wc)` 的返回值。
- musl 当前实现中 `locale_t l` 参数**被忽略**，所有 locale 下行为一致。

### 不变量

**[Invariant]:** 纯函数，无副作用。`l` 参数预留但未使用（musl 不支持 per-locale 宽字符分类）。

### 意图

POSIX locale-aware 字符分类函数的内部实现桩。musl 的宽字符分类基于 Unicode 且与 locale 无关，因此 `iswalnum_l_impl` 直接委托给 `iswalnum`，忽略 locale 参数。该设计确保与 POSIX 接口兼容的同时保持实现简洁。

---

## iswalnum_l (对外导出)

**[Visibility]: External** — POSIX.1-2008 标准函数，`<wctype.h>` 声明。

### 前置/后置条件

同 `iswalnum_l_impl`，完全一致的语义。Rust 中 `iswalnum_l` 为独立的 `extern "C"` 函数，内部委托给 `iswalnum_l_impl`（相当于 C 中 `weak_alias` 的效果）。

### 不变量

**[Invariant]:** `iswalnum_l` 内部委托给 `iswalnum_l_impl`，两者行为完全一致。对外部 C 调用者透明等价。

---

## 跨文件依赖摘要

| 依赖符号 | 来源文件 | 可见性 | 说明 |
|---|---|---|---|
| `iswdigit` | `src/ctype/iswdigit.rs` | External | 见 `src/ctype/rust-spec/iswdigit.md` |
| `iswalpha` | `src/ctype/iswalpha.rs` | External | 见 `src/ctype/rust-spec/iswalpha.md` |
| `wint_t` | 本模块定义 (`c_uint` 别名) | 系统类型 | `c_uint` 的 pub type 别名 |
| `locale_t` | 本模块引用 (`*mut __locale_struct`) | 系统类型 | `*mut __locale_struct` 的 pub type 别名 |
| `WEOF` | 本模块定义 | 模块常量 | 值为 `0xffffffff_u32` |

---

## 依赖汇总

```
[RELY]
Predefined Types:
  c_int                     // core::ffi::c_int, 对应 C 的 int (函数返回类型)
  c_uint                    // core::ffi::c_uint, 对应 C 的 unsigned int
  wint_t                    // pub type wint_t = c_uint;
  __locale_struct           // 不透明结构体, 见 src/internal/locale_impl.h
                            // Rust: #[repr(C)] struct __locale_struct { _opaque: [u8; 1] }
  locale_t                  // pub type locale_t = *mut __locale_struct;
  WEOF                      // pub const WEOF: wint_t = 0xffffffff_u32;

Predefined Functions (External, extern "C" 导出):
  iswdigit(wc: wint_t) -> c_int  // 外部依赖: src/ctype/iswdigit.rs, 判断宽字符是否为十进制数字
  iswalpha(wc: wint_t) -> c_int  // 外部依赖: src/ctype/iswalpha.rs, 判断宽字符是否为 Unicode 字母

[GUARANTEE]
Exported Interface (extern "C", ABI 兼容):
  pub extern "C" fn iswalnum(wc: wint_t) -> c_int;
                              // 本模块保证对外提供的 iswalnum 接口签名与行为
  pub extern "C" fn iswalnum_l(wc: wint_t, l: locale_t) -> c_int;
                              // 本模块保证对外提供的 iswalnum_l 接口签名与行为
                              // 注: l 参数被忽略, 行为等效于 iswalnum(wc)

Internal Interface (不对外导出):
  pub(crate) fn iswalnum_l_impl(wc: wint_t, l: locale_t) -> c_int;
                              // 内部实现函数, 作为 iswalnum_l 的委托目标
                              // C 中对应 __iswalnum_l, Rust 中重命名为 iswalnum_l_impl
```