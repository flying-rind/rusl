# wcwidth (Rust Spec)

## 复杂度分级: Level 3

---

## [RELY]

Predefined Types & Functions:
  type c_int = i32;                           // 依赖1: C ABI int 类型 (core::ffi::c_int)
  type c_uchar = u8;                          // 依赖2: C ABI unsigned char 类型
  type wchar_t = core::ffi::c_int;            // 依赖3: 宽字符类型 (Linux 上为 i32)

  // 以下为编译时常量数据表，可在 Rust 侧自由重新组织
  // [Visibility]: Internal -- 编译时常量，不对外导出
  //
  // 来自 nonspacing.h: 组合字符（nonspacing mark）位图
  // 每个 bit 对应一个 Unicode 码点是否为组合字符（占 0 列宽）
  static TABLE: &[u8];  // 依赖4: nonspacing 字符位图
  //
  // 来自 wide.h: 宽字符（CJK 等占 2 列宽）位图
  // 每个 bit 对应一个 Unicode 码点是否为宽字符
  static WTABLE: &[u8]; // 依赖5: wide 字符位图

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
// 类型别名定义
/// wchar_t: 宽字符类型，Linux x86_64 上为 int (i32)
pub type wchar_t = core::ffi::c_int;

extern "C" {
    /// ISO C 标准库: 确定宽字符在终端显示时占用的列数
    /// 返回: 0（组合字符/null）、1（普通字符）、2（宽字符如 CJK）、-1（不可打印）
    /// [Visibility]: Exported (ISO C 标准库 <wchar.h>)
    pub fn wcwidth(wc: wchar_t) -> c_int;
}
```

Internal Symbols (不对外导出，可自由重新设计):

```rust
/// nonspacing 字符位图 — 来自 nonspacing.h
/// [Visibility]: Internal (不导出) — 对应 C 的 static const table[]
/// Rust 设计: 存储为 &'static [u8] 或 const 数组，利用 Rust 不可变静态安全保证
pub(crate) static NS_TABLE: &[u8] = &[...];

/// wide 字符位图 — 来自 wide.h
/// [Visibility]: Internal (不导出) — 对应 C 的 static const wtable[]
/// Rust 设计: 存储为 &'static [u8] 或 const 数组，利用 Rust 不可变静态安全保证
pub(crate) static WIDE_TABLE: &[u8] = &[...];
```

注: C spec 中 `table[]` 和 `wtable[]` 通过 `#include "nonspacing.h"` 和 `#include "wide.h"` 引入为 `static const unsigned char` 数组。在 Rust 中，可使用构建脚本 (`build.rs`) 将这两张表转换为 Rust 常量数组，或使用 `include!` 宏嵌入 Rust 格式的字节数据。

---

## 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wchar_t`（`i32`），Unicode 码点值。

**[Post-condition]:**
- Case 1: `wc` 是 null 字符 (`L'\0'`) -> 返回 0。
- Case 2: `wc` 是可打印字符
  - `wc < 0xff` 且 `(wc+1 & 0x7f) >= 0x21`：返回 1
  - `wc` 是 BMP 普通宽度字符：返回 1
  - `wc` 是宽字符（CJK 等，在 `WIDE_TABLE` 位图中）：返回 2
- Case 3: `wc` 是组合字符（nonspacing mark，在 `NS_TABLE` 位图中）-> 返回 0。
- Case 4: `wc` 是不可打印字符
  - `wc` 在 `0x01`-`0x1F` 或 `0x7F`-`0x9F` 范围内（C0/C1 控制字符）：返回 -1
  - `(wc & 0xfffe) == 0xfffe`（非字符码点）：返回 -1
  - `U+E0001` 或 `U+E0020`-`U+E00EF` 范围内的标记字符：返回 0

---

## 不变量

**[Invariant]:**
- `NS_TABLE`（nonspacing.h）和 `WIDE_TABLE`（wide.h）为编译时常量，运行时只读。
- Rust 实现中，这些表存储为 `&'static [u8]` 或 `const` 全局数组，利用 Rust 的不可变静态保证数据完整性。
- 函数为纯函数，线程安全。

---

## 设计说明

### ABI 兼容性

外部导出的 `wcwidth` 必须保持与 C ABI 完全兼容：
- `wc: wchar_t` 对应 C 的 `wchar_t`，在 Linux x86_64 上为 `int`（32 位有符号整数 `i32`）
- 返回值 `c_int` 对应 C 的 `int`（32 位有符号整数）
- 调用约定为 `extern "C"`

### 内部实现策略

C 实现通过 `#include` 引入两张位图表，使用裸指针和位运算进行查表。Rust 内部实现可自由重新设计：

1. **数据表重构**: C 中通过 `#include "nonspacing.h"` 和 `#include "wide.h"` 引入 `static const unsigned char table[]` 和 `static const unsigned char wtable[]`。Rust 中可使用：
   - 构建脚本 (`build.rs`) 在编译期解析 `.h` 文件并生成 Rust 格式的 `const` 数组
   - 使用 `include_bytes!` 宏嵌入预处理后的二进制数据
   - 将表存储为 `&'static [u8]`，享受 Rust 的 slice 边界检查

2. **查表逻辑优化**: C 使用宏和内联函数做位图查找。Rust 可封装为内联辅助函数：
   ```rust
   // 示意: 位图查找的内部辅助函数
   #[inline]
   fn bitmap_test(table: &[u8], codepoint: u32) -> bool {
       // codepoint / 8 获取字节索引, codepoint % 8 获取位偏移
       let byte_idx = codepoint as usize / 8;
       let bit_idx = codepoint as usize % 8;
       byte_idx < table.len() && (table[byte_idx] & (1 << bit_idx)) != 0
   }
   ```

3. **多级分类重构**: 将 C 的三阶段分类逻辑重构为 Rust 的 match 守卫或嵌套 if-else，使分类逻辑更清晰：
   ```
   match wc {
       0 => 0,                                          // null
       w if w <= 0x1F || (0x7F..=0x9F).contains(&w) => -1, // 控制字符
       w if (w & 0xfffe) == 0xfffe => -1,               // 非字符码点
       w if w < 0xff => 1,                              // ASCII 可打印
       w if bitmap_test(NS_TABLE, w) => 0,              // 组合字符
       w if bitmap_test(WIDE_TABLE, w) => 2,            // 宽字符
       // 高位平面处理...
       _ => 1,
   }
   ```

4. **零成本抽象**: 利用 `#[inline]` 和 `const fn` 确保位图查找在编译期尽可能内联展开，与 C 原实现性能相当

### no_std 兼容

- 使用 `core::ffi::c_int` 替代 `libc::c_int`
- 数据表存储在 `static` 或 `const` 段，不依赖堆分配
- 使用 `include_bytes!` 宏嵌入二进制数据，无需运行时文件 I/O
- 不依赖 `libc` crate