# isxdigit Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn isxdigit(c: core::ffi::c_int) -> core::ffi::c_int;

// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn isxdigit_l(c: core::ffi::c_int, l: *mut core::ffi::c_void) -> core::ffi::c_int;

// [Visibility]: Internal — 不对外导出
// isxdigit 和 isxdigit_l 均委托至此内部实现
pub(crate) fn __isxdigit_l(c: core::ffi::c_int) -> core::ffi::c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int` (`core::ffi::c_int`)，值必须可表示为 `unsigned char` 或等于 `EOF`（-1）。

**[Post-condition]:**
- Case 1: `c` 是十六进制数字字符（`'0'`-`'9'`、`'A'`-`'F'` 或 `'a'`-`'f'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:**
- 纯函数。无内部状态。
- 依赖 `isdigit` 检测十进制数字部分。
- `locale` 参数保留为 API 兼容占位，内部实现忽略。

### 意图

判断字符是否为十六进制数字字符。复用 `isdigit` 检测数字，用 `|32` 技巧统一大小写后检查字母范围。

### 系统算法

```
return isdigit(c) || ((unsigned)c|32)-'a' < 6;
先委派给 isdigit，若失败则通过 |32 将大写转小写后检查 'a'-'f'。
时间复杂度 O(1)。
```

### Rust 内部实现要点

- 直接嵌入 `isdigit` 的逻辑（内部调用 `__isdigit_l`），不使用外部 FFI 调用
- 实现方式：
  ```rust
  fn __isxdigit_l(c: c_int) -> c_int {
      let u = c as u8;  // 取低 8 位
      let is_digit = u.wrapping_sub(b'0') < 10;
      let is_alpha = (u | 32).wrapping_sub(b'a') < 6;
      (is_digit || is_alpha) as c_int
  }
  ```
- `isxdigit(c)` 等价于 `__isxdigit_l(c)`
- `isxdigit_l(c, _l)` 等价于 `__isxdigit_l(c)`
- 与宽字符版本 `iswxdigit` 的逻辑完全一致，仅输入类型从 `c_uint` 变为 `c_int`（取低 8 位）

---

/* Rely */
[RELY]
Predefined Types:
  (无外部类型依赖)

[GUARANTEE]
Exported Interface:
  extern "C" fn isxdigit(c: core::ffi::c_int) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 isxdigit 符号
  extern "C" fn isxdigit_l(c: core::ffi::c_int, l: *mut core::ffi::c_void) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 isxdigit_l 符号