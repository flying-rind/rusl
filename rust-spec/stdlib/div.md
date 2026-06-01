# div/ldiv/lldiv/imaxdiv Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
#[repr(C)]
pub struct div_t { pub quot: i32, pub rem: i32 }

#[repr(C)]
pub struct ldiv_t { pub quot: i64, pub rem: i64 }

#[repr(C)]
pub struct lldiv_t { pub quot: i64, pub rem: i64 }

#[repr(C)]
pub struct imaxdiv_t { pub quot: i64, pub rem: i64 }

#[no_mangle] pub unsafe extern "C" fn div(num: i32, den: i32) -> div_t;
#[no_mangle] pub unsafe extern "C" fn ldiv(num: i64, den: i64) -> ldiv_t;
#[no_mangle] pub unsafe extern "C" fn lldiv(num: i64, den: i64) -> lldiv_t;
#[no_mangle] pub unsafe extern "C" fn imaxdiv(num: i64, den: i64) -> imaxdiv_t;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
den 必须非零。num == TYPE_MIN && den == -1 时 UB。

**[Post-condition]:**
返回 { quot = num/den (向零截断), rem = num%den }，满足 num == quot*den + rem。

### 不变量

纯函数。无内部状态。