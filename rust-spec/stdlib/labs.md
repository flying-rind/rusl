# labs/llabs/imaxabs Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn labs(a: i64) -> i64;

#[no_mangle]
pub unsafe extern "C" fn llabs(a: i64) -> i64;

#[no_mangle]
pub unsafe extern "C" fn imaxabs(a: i64) -> i64;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
各函数接受对应类型的整数（除该类型 MIN 外）。

**[Post-condition]:**
返回绝对值。MIN 时行为未定义。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 系统算法

```
同 abs: if a > 0 { a } else { -a }
```