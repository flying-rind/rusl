# rand_r Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn rand_r(seed: *mut u32) -> i32;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seed`: 指向调用者维护的 u32 种子变量（非空指针）。

**[Post-condition]:**
- `*seed` 被更新为 `*seed * 1103515245 + 12345`。
- 返回 `(*seed_new) & RAND_MAX`。

### 不变量

**[Invariant]:** 所有状态由调用者通过 `seed` 参数管理，无全局状态。天然线程安全。

### 意图

可重入版本的 `rand()`。使用与 `rand()` 相同的 LCG 参数（但不共享状态）。

### 系统算法

```
*seed = (*seed).wrapping_mul(1103515245).wrapping_add(12345);
return (*seed & RAND_MAX) as i32;
```