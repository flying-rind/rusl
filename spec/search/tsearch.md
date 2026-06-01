# tsearch 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <search.h>

typedef struct node { ... } posix_tnode;

void *tsearch(const void *key, void **rootp, int (*compar)(const void *, const void *));
void *tfind(const void *key, void *const *rootp, int (*compar)(const void *, const void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`key`: 查找/插入键。`rootp`: 树根指针的指针。`compar`: 比较函数，语义同 qsort 的 cmp。

**[Post-condition]:**
### tsearch
- Case 1 已存在: 返回匹配节点的指针。
- Case 2 不存在: 分配新节点，插入树中，返回新节点指针。失败（ENOMEM）返回 NULL。

### tfind
- Case 1 找到: 返回匹配节点指针。
- Case 2 未找到: 返回 NULL（不修改树）。

### 不变量

**[Invariant]:** 树平衡不变量: musl 使用标准的无序二叉树（非 AVL 亦非红黑树），查找/插入最坏 O(n) 但平均 O(log n)。

### 意图

POSIX 标准二叉树搜索/插入。用于管理有序集合。musl 的实现为简单 BST（非自平衡），在有序插入退化为链表 O(n)。

### 系统算法

```
tsearch: 沿树搜索 key，若找到返回节点，否则 malloc 新节点插入正确位置。
tfind: 仅搜索，不插入。
```

## 依赖关系

### 依赖的函数
- `cmp()`: 调用者提供的比较函数（函数指针参数）
- `__tsearch_balance()`: AVL 再平衡函数（internal, 同文件定义）
- `height()`: 内部 inline 辅助函数（用于 __tsearch_balance）
- `rot()`: 内部 static 函数，执行 AVL 旋转操作
- `malloc()`: 分配新节点

### 依赖的数据结构
- `struct node`: 内部 AVL 树节点（定义于 tsearch.h），包含 key、a[2]（左右子节点）、h（高度）
- `MAXH`: 路径栈最大深度宏（定义于 tsearch.h）

### 依赖的外部资源
- `<search.h>`: 函数声明
- `<stdlib.h>`: malloc
- `"tsearch.h"`: 内部结构体定义

### 被依赖
- 应用层直接调用；tfind, tdelete, tdestroy, twalk 依赖其节点结构体定义
