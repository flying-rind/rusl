# iswpunct Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswpunct(wc: core::ffi::c_uint) -> core::ffi::c_int;

// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswpunct_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;

// [Visibility]: Internal — 不对外导出
// iswpunct 和 iswpunct_l 均委托至此内部实现
pub(crate) fn __iswpunct_l(c: core::ffi::c_uint, _l: Option<&Locale>) -> core::ffi::c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t` (`core::ffi::c_uint`)，任意宽字符值（含 `WEOF` / `0xFFFF_FFFFu32`）。

**[Post-condition]:**
- Case 1: `wc` 是标点符号
  - `wc < 0x20000U` 且二级位图查找命中：返回 1
- Case 2: `wc` 不是标点符号
  - `wc >= 0x20000U`：返回 0
  - `wc < 0x20000U` 但位图查找未命中：返回 0

### 不变量

**[Invariant]:**
- `table` 静态数组（来自 `punct.h` 编译时生成的 Unicode 标点位图数据）为只读常量。Rust 内部实现可重新设计其存储形式（如使用 Rust `const` 数组或 `static` 懒初始化），但不改变查找结果。
- 函数为纯函数，无副作用，线程安全。
- `locale` 参数在当前 musl 实现中未实际使用（Unicode 标点判定不依赖 locale），保留为 API 兼容占位。

### 意图

判断宽字符是否为 Unicode 标点符号。使用与 `iswalpha` 相同的二级位图查找结构，覆盖 BMP 及 Supplementary Multilingual Plane（到 U+1FFFF）的所有标点码点。

### 系统算法

```
Phase 1（二级位图查找）:
  wc < 0x20000 时:
    - table[wc>>8]: 获取高 8 位对应的二级表偏移
    - 索引 = table[高位] * 32 + ((wc & 255) >> 3)
    - 位掩码 = 1 << (wc & 7)
    - 返回位图命中结果

Phase 2（越界）:
  wc >= 0x20000 返回 0

时间复杂度 O(1)，使用编译时生成的位图表。
```

### Rust 内部实现要点

- `punct.h` 位图表数据在 Rust 侧可重新设计为 `const TABLE: &[u8]` 或模块级 `static` 数组
- 位图查找逻辑使用安全的 Rust 索引访问，`wc < 0x20000` 作为边界检查保证不越界
- `iswpunct(wc)` 等价于 `__iswpunct_l(wc, CURRENT_LOCALE)`
- `iswpunct_l(c, l)` 等价于 `__iswpunct_l(c, Some(locale_from_ptr(l)))`
- 内部实现可引入 `UnicodePunctTable` 结构体封装位图查找逻辑，提升模块内聚性

---

/* Rely */
[RELY]
Predefined Types:
  wint_t:        core::ffi::c_uint                  // 依赖1: 宽字符类型
  Locale:        内部 opaque 结构体                   // 依赖2: locale 内部类型
Internal Data:
  punct.h 位图表数据                                     // 依赖3: Unicode 标点符号位图数据（内部嵌入）

[GUARANTEE]
Exported Interface:
  extern "C" fn iswpunct(wc: core::ffi::c_uint) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswpunct 符号
  extern "C" fn iswpunct_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswpunct_l 符号