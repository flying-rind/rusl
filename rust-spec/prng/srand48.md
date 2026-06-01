# srand48 Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn srand48(seedval: i64);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seedval`: 任意 i64 值，仅低 32 位用于设置种子。

**[Post-condition]:**
- 重置乘数和加数为默认 LCG 参数。
- 设置新种子为 {0x330E, (seedval & 0xFFFF) as u16, ((seedval >> 16) & 0xFFFF) as u16}。

### 不变量

**[Invariant]:** 修改全局状态，非线程安全。

### 意图

通过 32 位种子值初始化 48 位 LCG。

### 系统算法

```
重置全局 mult 和 add 为默认值；设置 __seed48[0..2] 为新种子值。
```