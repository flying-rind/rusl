# strtol 族 Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle] pub unsafe extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn strtoul(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64;
#[no_mangle] pub unsafe extern "C" fn strtoull(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64;
#[no_mangle] pub unsafe extern "C" fn strtoimax(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> i64;
#[no_mangle] pub unsafe extern "C" fn strtoumax(s: *const c_char, endptr: *mut *mut c_char, base: i32) -> u64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 待解析字符串。
- `base`: 0 或 2-36。0 时自动检测（0x→16, 0→8, else 10）。

**[Post-condition]:**
- 成功: 返回转换值，*endptr 指向首个非数字字符。
- 无数字: 返回 0，*endptr = s。
- 溢出: 返回极值 + errno = ERANGE。

### 不变量

委托给内部 `__intscan` 引擎。6 个函数共享 strtox 模板。

### 系统算法

```
构建 FILE 包装器 -> __intscan(&f, base, 1, lim) -> 
逐字符累加 + 溢出检测 (cutoff/cutlim)
```