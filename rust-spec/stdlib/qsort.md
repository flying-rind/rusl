# qsort/qsort_r Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: Public
type CmpFun = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;
type CmpFunR = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> i32;

#[no_mangle] pub unsafe extern "C" fn qsort(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFun>);
#[no_mangle] pub unsafe extern "C" fn qsort_r(base: *mut c_void, nel: usize, width: usize, cmp: Option<CmpFunR>, arg: *mut c_void);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `base`: nel * width 可读写内存。
- `cmp`: 比较函数，不得修改元素。

**[Post-condition]:**
- 数组按 cmp 升序原地排序。排序不稳定。
- qsort 通过 wrapper_cmp 适配到 qsort_r。

### 不变量

使用 Smoothsort 算法。Leonardo 堆 + 双字位运算。

### 系统算法

```
基于 Smoothsort (自适应 Heapsort):
- 构建阶段: 逐元素建 Leonardo 堆森林
- 整理阶段: trinkle 跨堆合并 + sift 筛选
- 时间复杂度: 最坏 O(n log n)，接近有序 O(n)
- 空间复杂度: O(1)
```