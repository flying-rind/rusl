# tfind 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <search.h>

void *tfind(const void *key, void *const *rootp, int (*compar)(const void *, const void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 tsearch 参数约定。

**[Post-condition]:**
- Case 1 找到: 返回匹配节点指针。
- Case 2 未找到: 返回 NULL。

### 不变量

**[Invariant]:** 纯函数。不修改树。

### 意图

在二叉树中搜索指定 key。tsearch 的只读版本，不插入新节点。

### 系统算法

```
沿树二分查找 key，找到返回节点，否则返回 NULL。
```

## 依赖关系

### 依赖的函数
- `cmp()`: 调用者提供的比较函数（函数指针参数）

### 依赖的数据结构
- `struct node`: 内部 AVL 树节点（定义于 tsearch.h）

### 依赖的外部资源
- `<search.h>`: 函数声明
- `"tsearch.h"`: 内部结构体定义

### 被依赖
- 应用层直接调用
