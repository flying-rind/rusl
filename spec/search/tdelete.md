# tdelete 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <search.h>

void *tdelete(const void *restrict key, void **restrict rootp, int (*compar)(const void *, const void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 tsearch 的参数约定。`rootp` 指向非空树的根指针。

**[Post-condition]:**
- Case 1 找到并删除: 返回被删除节点的父节点指针；若删除根节点，返回指向新根的指针。
- Case 2 未找到: 返回 NULL，树结构不变。
- 释放被删除节点对应的内存。

### 不变量

**[Invariant]:** 删除操作可能改变树的根。若 key 指向的是树中节点自身的 key 字段，删除后 key 成为悬空指针。

### 意图

从二叉树中查找并删除指定 key 的节点。节点删除后内存被 free，调用者不应再引用返回的被删节点指针。

### 系统算法

```
沿树搜索 key:
  - 未找到 → return NULL
  - 找到节点 q:
    - q 无子或单子 → 用子节点替换 q
    - q 有双子 → 找后继 r（左子树最右或右子树最左），替换 q 的 key/data，删除 r
  - free(q) 并返回其父节点
```

## 依赖关系

### 依赖的函数
- `cmp()`: 调用者提供的比较函数（函数指针参数）
- `__tsearch_balance()`: AVL 再平衡函数（internal, tsearch.c）
- `free()`: 释放被删除节点内存

### 依赖的数据结构
- `struct node`: 内部 AVL 树节点（定义于 tsearch.h）
  - `key`: 指向用户数据的指针
  - `a[2]`: 左右子节点指针
  - `h`: 节点高度

### 依赖的外部资源
- `<search.h>`: 函数声明
- `<stdlib.h>`: free
- `"tsearch.h"`: 内部结构体定义和 __tsearch_balance 声明

### 被依赖
- 应用层直接调用
