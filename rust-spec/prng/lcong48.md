# lcong48 Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn lcong48(p: *const u16);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`p`: 指向 7 个 u16 的数组。p[0..2] = 新种子，p[3..5] = 新乘数，p[6] = 新加数。

**[Post-condition]:**
设置全局 LCG 的全部参数为 p 指定的值。

### 不变量

**[Invariant]:** 直接修改全局状态，非线程安全。

### 意图

一次性设置 48 位 LCG 的全部参数（种子、乘数、加数）。

### 系统算法

```
__seed48[0..2] = p[0..2]; mult[0..2] = p[3..5]; add = p[6];
```