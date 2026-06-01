# twalk Rust 接口

## 复杂度分级: Level 1

---

## Rust 接口

```rust
// [Visibility]: Public
#[repr(C)]
pub enum VISIT {
    preorder = 0,
    postorder = 1,
    endorder = 2,
    leaf = 3,
}

#[no_mangle]
pub unsafe extern "C" fn twalk(
    root: *const c_void,
    action: Option<unsafe extern "C" fn(*const c_void, VISIT, i32)>,
);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `root`: 树根指针（可为 null）。
- `action`: 遍历回调。参数: nodep 指向节点指针（实际为 &Node），which 为 VISIT 枚举，depth 为深度（根=0）。

**[Post-condition]:**
- 前序/中序/后序遍历 AVL 树。内部节点被调用 3 次（preorder, postorder, endorder），叶节点 1 次（leaf）。

### 不变量

**[Invariant]:** 只读遍历。不修改树结构。action 中不应修改树链接关系。

### 意图

AVL 树遍历回调。用于打印、统计等只读分析。

### 系统算法

```
fn walk(node, action, depth):
  if !node: return
  if node.h == 1: action(node, leaf, depth)
  else:
    action(node, preorder, depth)
    walk(node.left, action, depth+1)
    action(node, postorder, depth)
    walk(node.right, action, depth+1)
    action(node, endorder, depth)
```