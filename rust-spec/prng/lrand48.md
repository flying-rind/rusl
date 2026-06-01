# lrand48 Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn lrand48() -> i64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
无参数。使用全局状态。

**[Post-condition]:**
推进全局 LCG 一步，返回 [0, 2^31) 的非负 i64。

### 不变量

**[Invariant]:** 读取并修改全局状态，非线程安全。

### 意图

返回非负伪随机长整数的 48 位 LCG 生成器。

### 系统算法

```
调用 __rand48_step()，返回 (result >> 17) & 0x7FFFFFFF。
```