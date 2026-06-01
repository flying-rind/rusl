# twalk 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <search.h>

void twalk(const void *root, void (*action)(const void *nodep, VISIT which, int depth));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `root`: 树根指针。
- `action`: 遍历回调。`nodep` 指向当前节点指针，`which` 为 preorder/postorder/endorder/leaf，`depth` 为深度（根 = 0）。

**[Post-condition]:**
- 前序、中序、后序遍历二叉树，对每个节点（包括内部节点和叶子）多次调用 action。
- 内部节点被访问 3 次（preorder, postorder, endorder），叶子 1 次（leaf）。

### 不变量

**[Invariant]:** 只读遍历。不修改树结构。action 中不应修改树的链接关系否则后续遍历行为未定义。

### 意图

遍历二叉树并在每个节点的不同时机调用用户指定的动作函数。用于打印树结构、收集统计信息等只读分析任务。

### 系统算法

```
递归遍历：preorder 时 action(nodep, preorder, depth)→遍历左子树→postorder 时 action→遍历右子树→endorder 时 action。leaf 节点仅调用一次 (leaf)。
```

## 依赖关系

### 依赖的函数
- `walk()`: 内部 static 递归函数，执行实际遍历
- `action()`: 调用者提供的遍历回调（函数指针参数）

### 依赖的数据结构
- `struct node`: 内部 AVL 树节点（定义于 tsearch.h）
- `VISIT` 枚举: preorder/postorder/endorder/leaf（定义于 search.h）

### 依赖的外部资源
- `<search.h>`: 函数声明、VISIT 枚举
- `"tsearch.h"`: 内部结构体定义

### 被依赖
- 应用层直接调用
