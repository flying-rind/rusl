# toupper (Rust Spec)

## 复杂度分级: Level 1

---

## [RELY]

Predefined Types & Functions:
  type c_int = i32;                           // 依赖1: C ABI int 类型 (core::ffi::c_int)
  type c_void = core::ffi::c_void;             // 依赖2: C ABI void 类型
  fn islower(c: c_int) -> c_int;              // 依赖3: islower 实现 (src/ctype/islower.rs, Internal)

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
extern "C" {
    /// ISO C 标准库: 将小写字母转换为大写字母
    /// [Visibility]: Exported (ISO C 标准库 <ctype.h>)
    pub fn toupper(c: c_int) -> c_int;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 版本，行为与 toupper 相同（当前单 locale 实现）
    /// [Visibility]: Exported (POSIX 扩展 <ctype.h>)
    pub fn toupper_l(c: c_int, l: *mut c_void /* locale_t */) -> c_int;
}
```

Internal Symbols (不对外导出，可自由重新设计):

```rust
/// 内部 locale-aware 实现，忽略 locale 参数，直接委托 toupper
/// [Visibility]: Internal (不导出) -- 对应 C 的 __toupper_l
/// C spec 原标注: Internal -- __ 前缀内部实现
/// Rust 设计: 使用 pub(crate) 可见性，可直接内联为 toupper(c)
pub(crate) fn __toupper_l(c: c_int, _l: *mut c_void /* locale_t */) -> c_int;
```

注: `toupper_l` 在 C 中通过 `weak_alias(__toupper_l, toupper_l)` 实现。在 Rust 中如果无法直接使用 weak_alias，可在 `toupper_l` 的 `extern "C"` 函数体中直接调用 `__toupper_l` 或内联其逻辑。

---

## 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `c_int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是小写字母 (`'a'`-`'z'`)
  - 返回对应的大写字母 (`c & 0x5f`，等价于 `c - 32`)。
- Case 2: `c` 不是小写字母
  - 返回原值 `c`。

**[Post-condition]: `toupper_l` / `__toupper_l`**
- `__toupper_l(c_int c, *mut c_void l)` 忽略 `l` 参数，直接调用 `toupper(c)`。
- `toupper_l` 是 POSIX 的 locale-aware 版本接口，行为与 `toupper` 相同（当前 rusl 单 locale 实现）。

---

## 不变量

**[Invariant]:** 纯函数（无副作用，仅依赖输入参数）。线程安全（无共享可变状态）。

---

## 设计说明

### ABI 兼容性

外部导出的 `toupper` 和 `toupper_l` 必须保持与 C ABI 完全兼容：
- 参数类型 `c_int` 对应 C 的 `int`（32 位有符号整数）
- 返回值类型 `c_int` 对应 C 的 `int`
- 调用约定为 `extern "C"`
- `locale_t` 参数使用 `*mut c_void` 作为不透明指针，保持指针宽度 ABI 兼容

### 内部实现策略

`__toupper_l` 为内部符号，不需要保持与原 C 实现一致。可使用以下 Rust 优化：
- 直接将 `__toupper_l` 实现为 `pub(crate)` 辅助函数，内联委托给 `toupper`
- `toupper` 核心逻辑可使用 Rust 的安全模式实现：通过 `islower` 判断后用位运算 `c & 0x5f` 完成转换
- ASCII 编码中小写字母范围 `0x61`-`0x7a`，对应大写 `0x41`-`0x5a`，相差 `0x20`（bit5）。位运算 `c & 0x5f` 清除 bit5 即完成大写转换
- `locale_t` 参数在当前单 locale 实现中可安全忽略
- `toupper_l` 的 `weak_alias` 语义：若目标平台不支持 weak symbol，可在 `extern "C" fn toupper_l` 中直接调用 `toupper(c)` 达到等价效果

### no_std 兼容

- 使用 `core::ffi::c_int`、`core::ffi::c_void` 替代 `std::os::raw`
- 不依赖 `libc` crate