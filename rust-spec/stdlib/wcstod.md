# wcstod/wcstof/wcstold Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle] pub unsafe extern "C" fn wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64;
#[no_mangle] pub unsafe extern "C" fn wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32;
#[no_mangle] pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 以 null 结尾的宽字符串。
- `endptr`: 可为 null。

**[Post-condition]:**
同 strtod，操作对象为宽字符串。内部将宽字符串适配为 FILE 流后委托给 __floatscan。

### 不变量

纯函数。非 ASCII 宽字符映射为 '@'（作弊策略，仅用于数字解析）。

### 系统算法

```
自定义 do_read: wchar_t -> byte (非ASCII → '@') -> FILE 适配 -> __floatscan
```