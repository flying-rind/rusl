# tdelete Rust 接口

## 复杂度分级: Level 2

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn tdelete(
    key: *const c_void,
    rootp: *mut *mut c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 tsearch。`rootp` 指向非空树的根指针。

**[Post-condition]:**
- 找到并删除: 释放节点内存，AVL 再平衡。返回被删除节点的父节点指针（若删除根节点，返回指向新根的指针）。
- 未找到: 返回 null。

### 不变量

**[Invariant]:** 删除后保持 AVL 平衡性质。key 指向的用户数据不释放。

### 意图

从 AVL 树中查找并删除指定节点。删除后自动再平衡。

### 系统算法

```
1. 沿树搜索 key，记录路径栈 a[]
2. 若被删节点有双子 -> 找后继 -> 替换 key -> 删除后继
3. 用子节点替换被删节点 -> free(node)
4. 逐祖先 __tsearch_balance 恢复平衡
```