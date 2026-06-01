# llabs Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
#[no_mangle]
pub unsafe extern "C" fn llabs(a: i64) -> i64;
```

### 前置/后置条件

同 abs，类型为 i64。纯函数，返回绝对值。