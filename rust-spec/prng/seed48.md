# seed48 Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn seed48(seed16v: *const u16) -> *mut u16;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seed16v`: 指向 3 个 u16 的新种子值（非空指针）。

**[Post-condition]:**
- 设置新种子，重置 mult/add 为默认值。
- 返回指向静态缓冲区的指针（含旧种子值，后续调用会覆盖）。

### 不变量

**[Invariant]:** 静态缓冲区非线程安全。非可重入。

### 意图

设置 48 位 LCG 种子并返回旧种子。便于保存/恢复 LCG 状态。

### 系统算法

```
保存旧种子至 static buf；重置 mult/add；设置新种子；返回 buf.as_mut_ptr()
```