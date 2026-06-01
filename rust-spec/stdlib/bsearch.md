# bsearch Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nel: usize,
    width: usize,
    cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `base`: 已按 cmp 升序排列的数组。
- `cmp`: 比较函数（返回 <0/=0/>0）。

**[Post-condition]:**
- 找到: 返回匹配元素指针。
- 未找到: 返回 null。

### 不变量

纯函数。不修改数组。

### 系统算法

```
标准二分查找: while nel > 0 { mid = base + width*(nel/2); sign = cmp(key, mid); ... }
O(log n)
```