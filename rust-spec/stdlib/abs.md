# abs Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn abs(a: i32) -> i32;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`a`: 任意 i32 值（除 i32::MIN 外）。

**[Post-condition]:**
- `a >= 0`: 返回 a。
- `a < 0`: 返回 -a。
- `a == i32::MIN`: 行为未定义。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 系统算法

```
if a > 0 { a } else { -a }
```