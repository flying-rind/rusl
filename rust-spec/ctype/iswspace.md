# iswspace Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswspace(wc: core::ffi::c_uint) -> core::ffi::c_int;

// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswspace_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;

// [Visibility]: Internal — 不对外导出
// iswspace 和 iswspace_l 均委托至此内部实现
pub(crate) fn __iswspace_l(c: core::ffi::c_uint) -> bool;
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t` (`core::ffi::c_uint`)，任意宽字符值（含 `WEOF` / `0xFFFF_FFFFu32`）。

**[Post-condition]:**
- Case 1: `wc` 是空白字符
  - `wc` 非零且在预定义的空白字符列表 `SPACES` 中
  - 返回非零值。
  空白字符列表：`' '`, `'\t'`, `'\n'`, `'\r'`, `'\v'`, `'\f'`, U+0085, U+2000-U+2006, U+2008-U+200A, U+2028, U+2029, U+205F, U+3000
- Case 2: `wc` 不是空白字符或 `wc == 0`
  - `wc == 0` 直接返回 0（防止 `wcschr` 将 `'\0'` 误匹配到 `SPACES` 数组的终止符）
  - 其他情况：字符未在 `SPACES` 列表中，返回 0

### 不变量

**[Invariant]:**
- `SPACES` 为静态只读常量数组（22 个空白字符码点）。
- 函数为纯函数。
- `wc == 0` 特殊处理防止搜索函数将空字符误匹配到列表终止符。
- `locale` 参数在当前 musl 实现中未实际使用，保留为 API 兼容占位。

### 意图

判断宽字符是否为 Unicode White_Space 属性的空白字符。排除了不间断空格（U+00A0, U+2007, U+202F）和非空白字形的脚本特定字符（U+1680, U+180E）。

### 系统算法

```
if (wc == 0) return 0;  // 防止 wcschr 将 '\0' 匹配到 SPACES 数组终止符
在 SPACES 数组中线性搜索 wc;
时间复杂度 O(n)，n = 22（常量）。
```

### Rust 内部实现要点

- `SPACES` 数组在 Rust 侧可定义为 `const SPACES: [u32; 22]`，使用安全的迭代器方法（如 `SPACES.contains(&wc)` 或 `SPACES.iter().any(|&x| x == wc)`）替代 C 的 `wcschr`
- 由于数组很小（22 个元素），线性扫描足够高效；若将来需要优化，可使用 `match` 或编译时生成的 `phf` 完美哈希
- `iswspace(wc)` 等价于 `__iswspace_l(wc) as c_int`
- `iswspace_l(c, _l)` 等价于 `__iswspace_l(c) as c_int`
- 不再需要依赖外部的 `wcschr`，Rust 迭代器提供等价功能且更安全

---

/* Rely */
[RELY]
Predefined Types:
  wint_t:        core::ffi::c_uint                  // 依赖1: 宽字符类型

[GUARANTEE]
Exported Interface:
  extern "C" fn iswspace(wc: core::ffi::c_uint) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswspace 符号
  extern "C" fn iswspace_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswspace_l 符号