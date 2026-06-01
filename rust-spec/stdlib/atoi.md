# atoi/atol/atoll Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle] pub unsafe extern "C" fn atoi(s: *const c_char) -> i32;
#[no_mangle] pub unsafe extern "C" fn atol(s: *const c_char) -> i64;
#[no_mangle] pub unsafe extern "C" fn atoll(s: *const c_char) -> i64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`s`: 以 null 结尾的字符串指针。

**[Post-condition]:**
- 跳过前导空白，解析可选符号和十进制数字，返回对应的整数值。
- 无有效数字返回 0，溢出行为未定义。

### 不变量

采用**负向累加**策略（中间值始终 <= 0），安全解析 TYPE_MIN。

### 系统算法

```
跳过 isspace -> 检测符号 -> 负向累加: n = 10*n - digit -> neg ? n : -n
```