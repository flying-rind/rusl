# tdestroy 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#define _GNU_SOURCE
#include <search.h>

void tdestroy(void *root, void (*free_node)(void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `root`: 树根节点指针。
- `free_node`: 回调函数，用于释放每个节点中的用户数据（可为 NULL）。

**[Post-condition]:**
- 后序遍历整个二叉树，对每个节点调用 `free_node(node->data)` 后再 free 节点本身。
- 树被完全销毁，root 成为悬空指针。

### 不变量

**[Invariant]:** 整个树被销毁后不可再使用。free_node 不得抛出异常或 longjmp。

### 意图

递归销毁整个二叉树，释放所有节点。GNU 扩展（非 POSIX），用于配对 tsearch 系列。

### 系统算法

```
后序遍历树：先销毁左子树，再右子树，然后 free_node(node_data)，最后 free(node)。
```

## 依赖关系

### 依赖的函数
- `freekey()`: 调用者提供的释放回调（函数指针参数，可为 NULL）
- `tdestroy()`: 自身递归调用（左右子树）
- `free()`: 释放节点内存

### 依赖的数据结构
- `struct node`: 内部 AVL 树节点（定义于 tsearch.h）

### 依赖的外部资源
- `<search.h>`: 函数声明
- `<stdlib.h>`: free
- `"tsearch.h"`: 内部结构体定义

### 被依赖
- 应用层直接调用
