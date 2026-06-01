# __seed48 Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Internal — rusl crate 内部实现
pub(crate) static mut __seed48: [u16; 7] = [0, 0, 0, 0xe66d, 0xdeec, 0x5, 0xb];
```

### 前置/后置条件

**[Visibility]:** Internal (pub(crate))

**[Pre-condition]:**
无。全局静态变量，默认初始化为标准 LCG 种子。

**[Post-condition]:**
该数组存储 drand48 族的全局种子值。索引 [0..2] 为 48 位种子，[3..5] 为默认乘数，[6] 为默认加数。

### 不变量

**[Invariant]:** 全局可变状态。C 中通过 TLS 访问，Rust 中使用 `static mut`。非线程安全。

### 意图

存储 48 位 LCG 的全局种子和参数。对应 C 中的 `unsigned short __seed48[7]` (hidden)。

### 系统算法

```
静态可变数组，由 drand48 族函数读写。
```