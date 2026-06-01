# wcswidth (Rust Spec)

## 复杂度分级: Level 1

---

## [RELY]

Predefined Types & Functions:
  type c_int = i32;                           // 依赖1: C ABI int 类型 (core::ffi::c_int)
  type c_uint = u32;                          // 依赖2: C ABI unsigned int 类型
  type wchar_t = core::ffi::c_int;            // 依赖3: 宽字符类型 (Linux 上为 i32)
  type size_t = usize;                        // 依赖4: 大小类型 (core::ffi::c_ulong / usize)
  fn wcwidth(wc: wchar_t) -> c_int;           // 依赖5: wcwidth 实现 (src/ctype/wcwidth.rs, Internal)

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
// 类型别名定义
/// wchar_t: 宽字符类型，Linux x86_64 上为 int (i32)
pub type wchar_t = core::ffi::c_int;

extern "C" {
    /// ISO C 标准库: 计算宽字符串的显示列宽总和
    /// [Visibility]: Exported (ISO C 标准库 <wchar.h>)
    pub fn wcswidth(wcs: *const wchar_t, n: size_t) -> c_int;
}
```

Internal Symbols (不对外导出，可自由重新设计):

本模块仅含 `wcswidth` 一个对外导出函数。其内部实现可直接使用 Rust 的安全迭代模式，不暴露额外的内部符号。内部辅助逻辑无需在 C spec 级别的内部符号之外单独声明。

---

## 前置/后置条件

**[Pre-condition]:**
- `wcs`: 指向以 null 结尾的宽字符串的指针（为 NULL 时行为未定义）。
- `n`: 最多检查的字符数（`size_t` 类型，即 `usize`）。

**[Post-condition]:**
- Case 1: 所有 `n` 个字符（或到终止 null）都可打印且列宽已知
  - 返回累计的列宽总和（非负整数 `c_int`）。
- Case 2: 遇到不可打印字符（`wcwidth` 返回 -1）
  - 提前终止，返回 -1。

---

## 不变量

**[Invariant]:** 纯函数。不修改 `wcs` 指向的内容。线程安全。

---

## 设计说明

### ABI 兼容性

外部导出的 `wcswidth` 必须保持与 C ABI 完全兼容：
- `wcs: *const wchar_t` 对应 C 的 `const wchar_t *`（指向宽字符串的只读指针）
- `n: size_t` 对应 C 的 `size_t`（在 64 位 Linux 上为 `unsigned long`，可用 `usize`）
- 返回值 `c_int` 对应 C 的 `int`（32 位有符号整数）
- 调用约定为 `extern "C"`

### 内部实现策略

C 实现使用裸指针遍历的 while 循环。Rust 内部实现可自由重新设计：

1. **安全指针转换**: 将 `*const wchar_t` 转换为 Rust 切片或迭代器，在 `unsafe` 边界完成指针操作，核心逻辑在安全 Rust 中实现

2. **Rust 风格实现**:
   ```rust
   // 示意: 使用迭代器风格的内部实现
   pub(crate) fn wcswidth_impl(wcs: *const wchar_t, n: usize) -> c_int {
       let mut total: c_int = 0;
       let mut count = 0usize;
       unsafe {
           let mut p = wcs;
           while count < n && *p != 0 {
               let w = wcwidth(*p);
               if w < 0 { return -1; }
               total += w;
               p = p.add(1);
               count += 1;
           }
       }
       total
   }
   ```
   
   更优的 Rust 设计可使用 `core::slice::from_raw_parts` 配合迭代器，在 unsafe 块内完成指针遍历后转入安全域。

3. **零成本抽象**: 内部循环可使用迭代器链和 `try_fold` 实现提前终止，与 C 的 while 循环性能相当

### no_std 兼容

- 使用 `core::ffi::*` 类型替代 `libc` 类型定义
- 不依赖 `libc` crate
- 不进行堆分配