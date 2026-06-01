# wctrans / towctrans (Rust Spec)

## 复杂度分级: Level 2

---

## [RELY]

Predefined Types & Functions:
  type c_int = i32;                                // 依赖1: C ABI int 类型 (core::ffi::c_int)
  type c_uint = u32;                               // 依赖2: C ABI unsigned int 类型 (wint_t)
  type c_ulong = u64;                              // 依赖3: C ABI unsigned long 类型 (wctrans_t, 64-bit)
  type c_char = i8;                                // 依赖4: C ABI char 类型
  type c_void = core::ffi::c_void;                  // 依赖5: C ABI void 类型
  fn towlower(wc: wint_t) -> wint_t;               // 依赖6: towlower 实现 (src/ctype/towctrans.rs, Exported)
  fn towupper(wc: wint_t) -> wint_t;               // 依赖7: towupper 实现 (src/ctype/towctrans.rs, Exported)
  fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;  // 依赖8: strcmp 实现 (src/string/strcmp.rs, Internal)

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
// 类型别名定义
/// wint_t: 宽字符整数类型，Linux 上为 unsigned int (32-bit)
pub type wint_t = core::ffi::c_uint;

/// wctrans_t: 大小写变换描述符类型，musl 中为 unsigned long
/// 值为 0（无效）、1（toupper）、2（tolower）
pub type wctrans_t = core::ffi::c_ulong;

extern "C" {
    /// ISO C 标准库: 将变换名称字符串解析为变换描述符
    /// class: "toupper" 或 "tolower"，以 null 结尾的 C 字符串
    /// 返回: wctrans_t 描述符（1/2），无效名称返回 0
    /// [Visibility]: Exported (ISO C 标准库 <wctype.h>)
    pub fn wctrans(class: *const c_char) -> wctrans_t;
}
```

```rust
extern "C" {
    /// ISO C 标准库: 根据描述符执行大小写变换
    /// trans == 1 -> towupper(wc), trans == 2 -> towlower(wc), 其他 -> wc
    /// [Visibility]: Exported (ISO C 标准库 <wctype.h>)
    pub fn towctrans(wc: wint_t, trans: wctrans_t) -> wint_t;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 变换名解析
    /// [Visibility]: Exported (POSIX 扩展 <wctype.h>)
    pub fn wctrans_l(class: *const c_char, l: *mut c_void /* locale_t */) -> wctrans_t;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 变换执行
    /// [Visibility]: Exported (POSIX 扩展 <wctype.h>)
    pub fn towctrans_l(wc: wint_t, trans: wctrans_t, l: *mut c_void /* locale_t */) -> wint_t;
}
```

Internal Symbols (不对外导出，可自由重新设计):

```rust
/// 内部 locale-aware 变换名解析实现，忽略 locale 参数
/// [Visibility]: Internal (不导出) -- 对应 C 的 __wctrans_l
/// Rust 设计: pub(crate) 可见性，可直接内联为 wctrans(class)
pub(crate) fn __wctrans_l(class: *const c_char, _l: *mut c_void /* locale_t */) -> wctrans_t;

/// 内部 locale-aware 变换执行实现，忽略 locale 参数
/// [Visibility]: Internal (不导出) -- 对应 C 的 __towctrans_l
/// Rust 设计: pub(crate) 可见性，可直接内联为 towctrans(wc, trans)
pub(crate) fn __towctrans_l(wc: wint_t, trans: wctrans_t, _l: *mut c_void /* locale_t */) -> wint_t;
```

注: `wctrans_l` / `towctrans_l` 在 C 中通过 `weak_alias` 实现。在 Rust 中如果无法直接使用 weak_alias，可在对应 `extern "C"` 函数体中直接调用 `__wctrans_l` / `__towctrans_l` 或内联其逻辑。

---

## 前置/后置条件

**[Pre-condition]:**

### wctrans
- `class`: 指向以 null 结尾的字符串，内容为 `"toupper"` 或 `"tolower"`。
- 若 `class` 为 NULL，行为未定义。

### towctrans
- `wc`: 类型为 `wint_t`，任意宽字符值。
- `trans`: 由 `wctrans()` 返回的变换描述符（`wctrans_t` 类型）。

**[Post-condition]:**

### wctrans
- Case 1: `class` 为 `"toupper"` -> 返回 `(wctrans_t)1`。
- Case 2: `class` 为 `"tolower"` -> 返回 `(wctrans_t)2`。
- Case 3: 其他 -> 返回 0。

### towctrans
- Case 1: `trans == 1`（"toupper"）-> 返回 `towupper(wc)`。
- Case 2: `trans == 2`（"tolower"）-> 返回 `towlower(wc)`。
- Case 3: 其他 -> 返回 `wc`。

### wctrans_l / towctrans_l
- 忽略 `locale_t` 参数，行为与 `wctrans` / `towctrans` 相同。

---

## 不变量

**[Invariant]:** 纯函数。变换描述符是固定整数（0、1、2），不与任何动态资源关联。线程安全。

---

## 设计说明

### ABI 兼容性

外部导出的 `wctrans`、`towctrans`、`wctrans_l`、`towctrans_l` 必须保持与 C ABI 完全兼容：
- `class: *const c_char` 对应 C 的 `const char *`（以 null 结尾的 C 字符串）
- `wint_t` 使用 `core::ffi::c_uint` (32-bit unsigned)
- `wctrans_t` 使用 `core::ffi::c_ulong`。在 musl 中 `wctrans_t` 定义为 `unsigned long`，在 64 位 Linux 上为 8 字节
- `locale_t` 参数使用 `*mut c_void` 作为不透明指针
- 调用约定为 `extern "C"`

### 内部实现策略

内部符号 `__wctrans_l`、`__towctrans_l` 及 `wctrans` 内部的字符串比较逻辑均可自由重新设计：

1. **字符串比较优化**: C 实现使用 `strcmp` 比较 `"toupper"` / `"tolower"`。Rust 内部可直接使用字节级别的模式匹配，例如：
   ```rust
   // 示意: 无需调用完整 strcmp，直接对已知字面量做快速匹配
   pub(crate) fn wctrans_impl(class: *const c_char) -> wctrans_t {
       // 内部实现可自包含，不依赖 strcmp 的完整符号
       // 使用不安全的字节比较内联完成匹配
   }
   ```

2. **变换分发**: C 实现使用 `if-else` 链。Rust 内部可使用 `match` 表达式，编译器会自动优化为跳转表：
   ```rust
   match trans {
       1 => towupper(wc),
       2 => towlower(wc),
       _ => wc,
   }
   ```

3. **locale 忽略**: `__wctrans_l` / `__towctrans_l` 的 `locale_t` 参数在当前单 locale 实现中忽略

4. **wctrans 内部字符串比较**: 虽然 C spec 中 `wctrans` 调用 `strcmp`，但在 Rust 重设计中，`wctrans` 的内部实现不需要依赖外部 `strcmp` 函数。可直接在 `wctrans` 内使用字节比较完成匹配，从而消除对外部 `strcmp` 的运行时依赖。如需要，可在依赖关系中保留对 `strcmp` 的可选引用

### no_std 兼容

- 使用 `core::ffi::*` 类型替代 `libc` 类型定义
- 不依赖 `libc` crate
- 不进行堆分配