# tdestroy Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public (GNU extension)
#[no_mangle]
pub unsafe extern "C" fn tdestroy(
    root: *mut c_void,
    free_key: Option<unsafe extern "C" fn(*mut c_void)>,
);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `root`: 树根节点指针（可为 null）。
- `free_key`: 释放用户数据的回调（可为 None，此时不处理 key）。

**[Post-condition]:**
- 后序遍历销毁整棵树：对每个节点先递归销毁子树，调用 free_key(key)，再 free(node)。
- root 成为悬空指针。

### 不变量

**[Invariant]:** 树被完全销毁后不可再用。free_key 中不应访问树结构。

### 意图

递归销毁整个 AVL 树并释放所有节点。GNU 扩展。

### 系统算法

```
fn destroy(node):
  if !node: return
  destroy(node.left)
  destroy(node.right)
  if free_key: free_key(node.key)
  free(node)
```