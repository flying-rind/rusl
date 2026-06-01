# towlower / towupper / casemap (Rust Spec)

## 复杂度分级: Level 3

---

## [RELY]

Predefined Types & Functions:
  type c_uint = u32;                          // 依赖1: C ABI unsigned int 类型 (wint_t)
  type c_void = core::ffi::c_void;             // 依赖2: C ABI void 类型
  // 以下为编译时常量数据表，来自 casemap.h，可在 Rust 侧自由重新组织
  // [Visibility]: Internal -- 编译时常量，不对外导出
  static tab: [u8; N];       // 依赖3: 二级基-6 分类表
  static rulebases: [T; M];  // 依赖4: 规则基址表
  static rules: [T; K];      // 依赖5: 规则数据表
  static exceptions: [T; E]; // 依赖6: 异常映射表

## [GUARANTEE]

Exported Interface (ABI 兼容，`extern "C"`):

```rust
// 类型别名定义 (pub 但不出现在共享库符号表中)
/// wint_t: 宽字符整数类型，Linux 上为 unsigned int (32-bit)
pub type wint_t = core::ffi::c_uint;

extern "C" {
    /// ISO C 标准库: 将宽字符转换为小写
    /// [Visibility]: Exported (ISO C 标准库 <wctype.h>)
    pub fn towlower(wc: wint_t) -> wint_t;
}
```

```rust
extern "C" {
    /// ISO C 标准库: 将宽字符转换为大写
    /// [Visibility]: Exported (ISO C 标准库 <wctype.h>)
    pub fn towupper(wc: wint_t) -> wint_t;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 小写转换
    /// [Visibility]: Exported (POSIX 扩展 <wctype.h>)
    pub fn towlower_l(wc: wint_t, l: *mut c_void /* locale_t */) -> wint_t;
}
```

```rust
extern "C" {
    /// POSIX.1-2008: locale-aware 大写转换
    /// [Visibility]: Exported (POSIX 扩展 <wctype.h>)
    pub fn towupper_l(wc: wint_t, l: *mut c_void /* locale_t */) -> wint_t;
}
```

Internal Symbols (不对外导出，可自由重新设计):

```rust
/// 内部 locale-aware 小写转换实现，忽略 locale 参数，直接委托 towlower
/// [Visibility]: Internal (不导出) -- 对应 C 的 __towlower_l
pub(crate) fn __towlower_l(wc: wint_t, _l: *mut c_void /* locale_t */) -> wint_t;

/// 内部 locale-aware 大写转换实现，忽略 locale 参数，直接委托 towupper
/// [Visibility]: Internal (不导出) -- 对应 C 的 __towupper_l
pub(crate) fn __towupper_l(wc: wint_t, _l: *mut c_void /* locale_t */) -> wint_t;

/// 核心大小写映射引擎
/// [Visibility]: Internal (不导出)
/// dir: 0 = 转小写 (towlower), 1 = 转大写 (towupper)
pub(crate) fn casemap(wc: wint_t, dir: c_int) -> wint_t;
```

注: `towlower_l` / `towupper_l` 在 C 中通过 `weak_alias` 实现。在 Rust 中如果无法直接使用 weak_alias，可在对应 `extern "C"` 函数体中直接调用 `__towlower_l` / `__towupper_l` 或内联其逻辑。

---

## 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`，即 `(wint_t)-1` / `0xFFFF_FFFF`）。

**[Post-condition]:**

### towlower
- Case 1: `wc` 存在小写映射 -> 返回对应的小写字符。
- Case 2: `wc` 不存在小写映射或 `wc == WEOF` -> 返回 `wc` 本身。

### towupper
- Case 1: `wc` 存在大写映射 -> 返回对应的大写字符。
- Case 2: `wc` 不存在大写映射或 `wc == WEOF` -> 返回 `wc` 本身。

### casemap (核心引擎)
```
casemap(wc, dir):
  Phase 1: wc >= 0x20000 -> 直接返回 wc（CJK Extension B 及以上无大小写）
  Phase 2: 使用二级基-6 表将字符分为 6 种类型
  Phase 3: 根据规则类型和数据执行映射
  Phase 4: rt < 2 -> 简单 delta 映射 (return wc + (rd & -(rt ^ dir)))
  Phase 5: 复杂映射时在 exceptions 数组中二分查找
时间复杂度: O(log n) 最坏情况，通常 O(1)。
```

---

## 不变量

**[Invariant]:**
- `tab`、`rulebases`、`rules`、`exceptions` 静态数组（来自 `casemap.h`）为编译时常量，程序生命周期内只读。
- Rust 实现中，这些表可存储为 `&'static [u8]` 或 `const` 全局数组，利用 Rust 的不可变静态安全保证。
- `casemap` 为纯函数，无副作用。
- `towlower` 和 `towupper` 仅通过 `dir` 参数（0/1）区分映射方向。
- 线程安全（无共享可变状态）。

---

## 设计说明

### ABI 兼容性

外部导出的 `towlower`、`towupper`、`towlower_l`、`towupper_l` 必须保持与 C ABI 完全兼容：
- `wint_t` 对应 C 的 `wint_t`，在 Linux x86_64 上为 `unsigned int`（32 位）。Rust 侧使用 `core::ffi::c_uint`。
- `WEOF` 定义为 `(wint_t)(-1)` = `0xFFFF_FFFF`，与 `core::ffi::c_uint` 的无符号语义兼容
- `locale_t` 参数使用 `*mut c_void` 作为不透明指针
- 调用约定为 `extern "C"`

### 内部实现策略

内部符号 `__towlower_l`、`__towupper_l`、`casemap` 以及所有 casemap 数据表均可自由重新设计：

1. **数据表重构**: C 中通过 `#include "casemap.h"` 引入静态数组。Rust 中可使用：
   - 构建脚本（`build.rs`）在编译期从 `casemap.h` 解析并生成 Rust 常量数组
   - 或使用 `include!` 宏直接嵌入预处理的 Rust 格式数据
   - 利用 Rust 的 `const` / `static` 保证运行时只读安全

2. **casemap 引擎优化**:
   - 用 Rust 的安全数组访问（边界检查在安全代码中自动执行）替代 C 的裸指针运算
   - 使用 `match` 表达式实现类型分发，替代 C 中的 switch-case
   - 二分查找可用标准库 `[T]::binary_search` 适配 no_std 环境
   - 内部辅助函数可标记为 `#[inline]` 以消除函数调用开销

3. **locale 忽略**: `__towlower_l` / `__towupper_l` 的 `locale_t` 参数在当前单 locale 实现中忽略，直接委托给 `towlower` / `towupper`

### no_std 兼容

- 使用 `core::ffi::c_uint` 替代 `libc::wint_t`
- 所有数据表存储在 `static` 或 `const` 段，不依赖堆分配
- 不依赖 `libc` crate