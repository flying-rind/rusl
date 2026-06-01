# tfind Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[no_mangle]
pub unsafe extern "C" fn tfind(
    key: *const c_void,
    rootp: *mut *const c_void,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> i32>,
) -> *mut c_void;
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 tsearch 参数约定。`rootp` 为 `*mut *const c_void`（不修改根指针）。

**[Post-condition]:**
- 找到: 返回匹配节点指针。
- 未找到: 返回 null（不修改树）。

### 不变量

**[Invariant]:** 只读操作。不修改树结构。

### 意图

在二叉树中搜索指定 key。tsearch 的只读版本。

### 系统算法

```
沿 AVL 树二分查找 key: 找到返回节点指针，否则返回 null。
```