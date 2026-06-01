# ecvt/fcvt/gcvt Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public (GNU extensions, 已过时)
#[no_mangle] pub unsafe extern "C" fn ecvt(x: f64, n: i32, dp: *mut i32, sign: *mut i32) -> *mut c_char;
#[no_mangle] pub unsafe extern "C" fn fcvt(x: f64, n: i32, dp: *mut i32, sign: *mut i32) -> *mut c_char;
#[no_mangle] pub unsafe extern "C" fn gcvt(x: f64, n: i32, b: *mut c_char) -> *mut c_char;
```

### 前置/后置条件

**[Visibility]:** Public (已过时)

**[Pre-condition]:**
- `ecvt(x, n, dp, sign)`: n 为有效数字位数（<= 15）。
- `fcvt(x, n, dp, sign)`: n 为小数位数（<= 1400）。
- `gcvt(x, n, b)`: b 为调用者提供的输出缓冲区。

**[Post-condition]:**
- `ecvt`: 返回静态缓冲区中的科学计数法数字串，*dp = 小数点位置，*sign = 符号。
- `fcvt`: 返回定点格式数字串或 ecvt 结果。过多前导零时返回 "000..." 常量字符串。
- `gcvt`: 将 %g 格式写入 b，返回 b。

### 不变量

ecvt 使用静态缓冲区（非线程安全）。fcvt 可能使用 ecvt 的缓冲区。

### 系统算法

```
ecvt: sprintf(tmp, "%.*e", n-1, x) -> 解析 -> atoi(exp) -> 返回
fcvt: sprintf(tmp, "%.*f", n, x) -> 计算 lz -> n<=lz ? "000..." : ecvt(x, n-lz, dp, sign)
gcvt: sprintf(b, "%.*g", n, x)
```

**注意：这些函数已过时，不推荐在新代码中使用。建议使用 sprintf/snprintf 代替。**