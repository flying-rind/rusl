# atof Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
#[no_mangle]
pub unsafe extern "C" fn atof(s: *const c_char) -> f64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`s`: 以 null 结尾的字符串指针。

**[Post-condition]:**
等价于 `strtod(s, null)`。无有效数字返回 0.0，溢出返回 ±HUGE_VAL。

### 不变量

纯函数。薄封装，完全委托给 strtod。

### 系统算法

```
strtod(s, ptr::null_mut())
```