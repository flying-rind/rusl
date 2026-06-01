# fcvt Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public (GNU extension, 已过时)
#[no_mangle]
pub unsafe extern "C" fn fcvt(x: f64, n: i32, dp: *mut i32, sign: *mut i32) -> *mut c_char;
```

### 前置/后置条件

**[Visibility]:** Public (已过时)

**[Pre-condition]:**
- `x`: 待转换的 f64。
- `n`: 小数位数（<= 1400）。
- `dp`: 输出小数点位置。
- `sign`: 输出符号（0=正，1=负）。

**[Post-condition]:**
- 返回定点格式小数部分字符串。
- 前导零过多时返回 "000000000000000" 常量。
- 其他情况委托给 ecvt。

### 不变量

大型格式化可能使用 ecvt 的静态缓冲区。

### 系统算法

```
sprintf(tmp, "%.*f", n, x) -> 计算 lz (strspn/strcspn) -> 
n <= lz ? "000000000000000"+14-n : ecvt(x, n-lz, dp, sign)
```