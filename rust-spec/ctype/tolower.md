# tolower (Rust Spec)

## 复杂度分级: Level 1

---

## [RELY]

Predefined Types & Functions:
  type c_int = i32;                           // 依赖1: C ABI int 类型 (core::ffi::c_int)
  type c_void = core::ffi::c_void;             // 依赖2: C ABI void 类型
  fn isupper(c: c_int) -> c_int;              // 依赖3: isupper 实现 (src/ctype/isupper.rs, Internal)

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
extern "C" {
    /// ISO C 标准库: 将大写字母转换为小写字母
    /// [Visibility]: Exported (ISO C 标准库 <ctype.h>)
    pub fn tolower(c: c_int) -> c_int;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 版本，行为与 tolower 相同（当前单 locale 实现）
    /// [Visibility]: Exported (POSIX 扩展 <ctype.h>)
    pub fn tolower_l(c: c_int, l: *mut c_void /* locale_t */) -> c_int;
}
```

Internal Symbols (不对外导出，可自由重新设计):

```rust
/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 tolower
/// [Visibility]: Internal (不导出) -- 对应 C 的 __tolower_l
/// C spec 原标注: Internal -- __ 前缀内部实现
/// Rust 设计: 使用 pub(crate) 可见性，可直接内联为 tolower(c)
pub(crate) fn __tolower_l(c: c_int, _l: *mut c_void /* locale_t */) -> c_int;
```

注: `tolower_l` 在 C 中通过 `weak_alias(__tolower_l, tolower_l)` 实现。在 Rust 中如果无法直接使用 weak_alias，可在 `tolower_l` 的 `extern "C"` 函数体中直接调用 `__tolower_l` 或内联其逻辑。

---

## 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `c_int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是大写字母 (`'A'`-`'Z'`)
  - 返回对应的小写字母 (`c | 32`，即 `c + 32`)。
- Case 2: `c` 不是大写字母
  - 返回原值 `c`。

**[Post-condition]: `tolower_l` / `__tolower_l`**
- `__tolower_l(c_int c, *mut c_void l)` 忽略 `l` 参数，直接调用 `tolower(c)`。
- `tolower_l` 是 POSIX 的 locale-aware 版本接口，行为与 `tolower` 相同（当前 rusl 单 locale 实现）。

---

## 不变量

**[Invariant]:** 纯函数。线程安全。不访问任何可变全局状态。

---

## 设计说明

### ABI 兼容性

外部导出的 `tolower` 和 `tolower_l` 必须保持与 C ABI 完全兼容：
- 参数类型 `c_int` 对应 C 的 `int`（32 位有符号整数）
- 返回值类型 `c_int` 对应 C 的 `int`
- 调用约定为 `extern "C"`
- `locale_t` 参数使用 `*mut c_void` 作为不透明指针，保持指针宽度 ABI 兼容

### 内部实现策略

`__tolower_l` 为内部符号，不需要保持与原 C 实现一致。可使用以下 Rust 优化：
- 直接将 `__tolower_l` 实现为 `pub(crate)` 辅助函数，内联委托给 `tolower`
- `tolower` 核心逻辑可使用 Rust 的安全模式实现：通过 `isupper` 判断后用位运算 `c | 32` 完成转换
- `locale_t` 参数在当前单 locale 实现中可安全忽略
- `tolower_l` 的 `weak_alias` 语义：若目标平台不支持 weak symbol，可在 `extern "C" fn tolower_l` 中直接调用 `tolower(c)` 达到等价效果

### no_std 兼容

- 使用 `core::ffi::c_int`、`core::ffi::c_void` 替代 `std::os::raw`
- 不依赖 `libc` crate