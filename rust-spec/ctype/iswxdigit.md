# iswxdigit Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswxdigit(wc: core::ffi::c_uint) -> core::ffi::c_int;

// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswxdigit_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;

// [Visibility]: Internal — 不对外导出
// iswxdigit 和 iswxdigit_l 均委托至此内部实现
pub(crate) fn __iswxdigit_l(c: core::ffi::c_uint) -> core::ffi::c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t` (`core::ffi::c_uint`)，任意宽字符值（含 `WEOF` / `0xFFFF_FFFFu32`）。

**[Post-condition]:**
- Case 1: `wc` 是十六进制数字字符（`'0'`-`'9'`、`'A'`-`'F'` 或 `'a'`-`'f'`）
  - 返回非零值。
- Case 2: 其他字符或 `WEOF`
  - 返回 0。

### 不变量

**[Invariant]:**
- 纯函数。无内部状态。
- 不依赖任何 locale 数据（十六进制数字在所有 locale 中定义相同）。
- `locale` 参数保留为 API 兼容占位，内部实现忽略。

### 意图

判断宽字符是否为十六进制数字字符。使用两个无符号区间检查：数字区间 `'0'`-`'9'` 和字母区间（通过 `|32` 统一大小写后检查 `'a'`-`'f'`）。

### 系统算法

```
return (unsigned)(wc-'0') < 10 || (unsigned)((wc|32)-'a') < 6;
第一项检查十进制数字（无符号减法下溢处理越界），第二项通过 |32 将大写转小写后检查 'a'-'f'。
时间复杂度 O(1)，无分支（使用布尔短路）。
```

### Rust 内部实现要点

- `__iswxdigit_l` 即上述位运算逻辑的直接 Rust 翻译，使用 `wrapping_sub` 或 `u32` 的普通减法（因为 Rust 在 release 模式下溢出为 wrapping）
- 更安全/可读的实现：
  ```rust
  fn __iswxdigit_l(c: u32) -> c_int {
      let is_digit = c.wrapping_sub('0' as u32) < 10;
      let is_alpha = (c | 32).wrapping_sub('a' as u32) < 6;
      (is_digit || is_alpha) as c_int
  }
  ```
- `iswxdigit(wc)` 等价于 `__iswxdigit_l(wc)`
- `iswxdigit_l(c, _l)` 等价于 `__iswxdigit_l(c)`

---

/* Rely */
[RELY]
Predefined Types:
  wint_t:        core::ffi::c_uint                  // 依赖1: 宽字符类型

[GUARANTEE]
Exported Interface:
  extern "C" fn iswxdigit(wc: core::ffi::c_uint) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswxdigit 符号
  extern "C" fn iswxdigit_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswxdigit_l 符号