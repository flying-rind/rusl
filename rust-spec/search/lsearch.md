# lsearch/lfind Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> i32;

#[no_mangle]
pub unsafe extern "C" fn lsearch(
    key: *const c_void,
    base: *mut c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<CmpFn>,
) -> *mut c_void;

#[no_mangle]
pub unsafe extern "C" fn lfind(
    key: *const c_void,
    base: *const c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<CmpFn>,
) -> *mut c_void;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `key`: 查找键的指针。
- `base`: 数组基地址。
- `nelp`: 元素个数指针（lsearch 会递增）。
- `width`: 每个元素字节大小。
- `compar`: 比较函数，返回 0 表示相等。

**[Post-condition]:**
- `lsearch`: 找到返回匹配元素指针；未找到则将 key 复制到数组末尾，`*nelp += 1`，返回新元素指针。
- `lfind`: 找到返回匹配元素指针；未找到返回 null（不修改数组）。

### 不变量

**[Invariant]:** lsearch 修改数组和 *nelp。lfind 只读。调用者保证 base 缓冲区有足够空间。

### 意图

无序数组线性搜索 + 自动追加（惰性去重集合）。POSIX 标准查找工具。

### 系统算法

```
遍历 base[0..*nelp]，用 compar(key, &base[i]) 逐一比较。
lsearch: 未找到时 ptr::copy_nonoverlapping(key, base.add(*nelp * width), width); *nelp += 1
lfind: 未找到返回 null
```