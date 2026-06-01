# imaxdiv Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
#[repr(C)]
pub struct imaxdiv_t { pub quot: i64, pub rem: i64 }

#[no_mangle]
pub unsafe extern "C" fn imaxdiv(num: i64, den: i64) -> imaxdiv_t;
```

### 前置/后置条件

同 div，类型为 intmax_t (i64)。纯函数。