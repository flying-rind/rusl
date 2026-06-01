# __qsort_r Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public (POSIX extension)
#[no_mangle]
pub unsafe extern "C" fn __qsort_r(base: *mut c_void, nel: usize, width: usize, cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> i32>, arg: *mut c_void);
```

### 前置/后置条件

**[Visibility]:** Public

同 qsort。额外 `arg` 参数透传给比较函数。nel <= 1 时直接返回。

### 不变量

入口函数，仅做参数校验。核心逻辑委托给 Smoothsort 实现（qsort.c）。

### 系统算法

```
if nel > 1 { Smoothsort(base, nel, width, cmp, arg) }
```