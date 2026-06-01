# __rand48_step Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Internal — rusl crate 内部实现，不对外导出
pub(crate) unsafe fn __rand48_step(xi: *mut u16, lc: *const u16) -> u64;
```

### 前置/后置条件

**[Visibility]:** Internal (pub(crate))

**[Pre-condition]:**
- `xi`: 指向 3 个 `u16` 的当前 48 位种子（小端排列），同时作为输出缓冲区。
- `lc`: 指向 4 个 `u16` 的 LCG 参数（前 3 个为乘数，第 4 个为加数）。

**[Post-condition]:**
- 执行 `X_new = (a * X_curr + c) mod 2^48`，结果写入 `xi[0..2]`。
- 返回 `X_new`（低 48 位）。

### 不变量

**[Invariant]:** 确定性纯函数。给定相同 `xi` 和 `lc` 产生相同输出。

### 意图

48 位 LCG 单步迭代核心引擎。C 版本为 hidden 符号，Rust 版本为 `pub(crate)` 内部可见。

### 系统算法

```
在 64 位无符号算术中计算 a * X_curr + c，结果自动截断至 48 位。
使用 as u16 转换确保移位正确性。
```