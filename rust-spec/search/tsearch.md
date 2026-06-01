# tsearch Rust 接口

## 复杂度分级: Level 3

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn tsearch(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `key`: 查找/插入键（仅存储指针值，不复制数据）。
- `rootp`: 树根指针的指针（`*rootp` 为 null 表示空树）。
- `compar`: 比较函数，返回 <0/=0/>0。

**[Post-condition]:**
- 已存在: 返回匹配节点指针。
- 不存在: malloc 新节点，AVL 插入并 rebalance，返回新节点指针。ENOMEM 时返回 null。

### 不变量

**[Invariant]:** AVL 自平衡树（非简单 BST）。节点高度差 <= 1。

### 意图

POSIX 二叉树搜索/插入。musl 实现使用的是 AVL 平衡树（非简单 BST）。

### 系统算法

```
沿树搜索 key -> 找到返回 -> 未找到 malloc Node { key, a=[0,0], h=1 } ->
插入并逐祖先 __tsearch_balance 恢复 AVL 性质
```