# iswupper Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswupper(wc: core::ffi::c_uint) -> core::ffi::c_int;

// [Visibility]: External — 对外导出，ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn iswupper_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;

// [Visibility]: Internal — 不对外导出
// iswupper 和 iswupper_l 均委托至此内部实现
pub(crate) fn __iswupper_l(c: core::ffi::c_uint, l: Option<&Locale>) -> core::ffi::c_int;
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t` (`core::ffi::c_uint`)，任意宽字符值（含 `WEOF` / `0xFFFF_FFFFu32`）。

**[Post-condition]:**
- Case 1: `wc` 是大写字母（`towlower(wc) != wc`，即存在对应的小写形式）
  - 返回非零值。
- Case 2: `wc` 不是大写字母（无对应小写或 `wc == WEOF`）
  - 返回 0。

### 不变量

**[Invariant]:**
- 纯函数。
- 依赖 `towlower` 的 Unicode 大小写映射表。
- `locale` 参数传递给 `towlower` 以支持 locale 感知的大小写转换，但当前 musl 实现中多为 C/POSIX locale。

### 意图

通过检测 `towlower(wc) != wc` 判断宽字符是否为大写字母。与 `iswlower` 对称，利用大小写转换表反向推断：若字符可被转为小写且结果不等于自身，则原字符为大写字母。

### 系统算法

```
return towlower(wc) != wc;
若字符有小写映射（且映射结果不等于自身），则为大写字母。
时间复杂度取决于 towlower 的 casemap 实现，通常为 O(1)。
```

### Rust 内部实现要点

- 核心依赖 `towlower` 函数。在 Rust 内部可设计为 `pub(crate) fn towlower_l(c: c_uint, l: Option<&Locale>) -> c_uint`
- `iswupper(wc)` 等价于 `(towlower_l(wc, CURRENT_LOCALE) != wc) as c_int`
- `iswupper_l(c, l)` 等价于 `(towlower_l(c, Some(locale_from_ptr(l))) != c) as c_int`
- 内部 `__iswupper_l` 即上述 `towlower_l` 调用 + 比较逻辑的组合

---

/* Rely */
[RELY]
Predefined Types:
  wint_t:        core::ffi::c_uint                  // 依赖1: 宽字符类型
  Locale:        内部 opaque 结构体                   // 依赖2: locale 内部类型
Predefined Functions:
  towlower(wint_t) -> wint_t                        // 依赖3: 宽字符转小写函数（核心依赖，由同模块或 casemap 子模块提供）

[GUARANTEE]
Exported Interface:
  extern "C" fn iswupper(wc: core::ffi::c_uint) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswupper 符号
  extern "C" fn iswupper_l(c: core::ffi::c_uint, l: *mut core::ffi::c_void) -> core::ffi::c_int;
                                  // 本模块保证对外提供与 C ABI 兼容的 iswupper_l 符号