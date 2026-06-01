# gcvt Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public (GNU extension, 已过时)
#[no_mangle]
pub unsafe extern "C" fn gcvt(x: f64, n: i32, b: *mut c_char) -> *mut c_char;
```

### 前置/后置条件

**[Visibility]:** Public (已过时)

**[Pre-condition]:**
- `x`: 待转换的 f64。
- `n`: 有效数字位数。
- `b`: 调用者提供的输出缓冲区。

**[Post-condition]:**
将 x 格式化为 %g 格式写入 b，返回 b。

### 不变量

纯函数。b 由调用者负责提供足够空间。

### 系统算法

```
sprintf(b, "%.*g", n, x); b
```