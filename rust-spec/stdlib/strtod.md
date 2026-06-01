# strtod/strtof/strtold Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle] pub unsafe extern "C" fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64;
#[no_mangle] pub unsafe extern "C" fn strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32;
#[no_mangle] pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 待解析字符串。
- `endptr`: 可为 null。非 null 时 `*endptr` 指向首个未解析字符。

**[Post-condition]:**
- 正常: 返回解析的浮点值。
- 无有效转换: 返回 0.0，`*endptr = s`。
- 溢出: 返回 ±HUGE_VAL，errno = ERANGE。

### 不变量

纯函数。委托给内部 `__floatscan` 引擎。

### 系统算法

```
构建 FILE 包装器 (sh_fromstring) -> __floatscan(&f, prec, 1) -> 
解析十进制/十六进制浮点数 -> 检测溢出/下溢
```