# lldiv Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
#[repr(C)]
pub struct lldiv_t { pub quot: i64, pub rem: i64 }

#[no_mangle]
pub unsafe extern "C" fn lldiv(num: i64, den: i64) -> lldiv_t;
```

### 前置/后置条件

同 div，类型为 i64。纯函数。