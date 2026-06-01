# insque 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <search.h>

void insque(void *element, void *pred);
void remque(void *element);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
### insque
- `element`: 待插入节点（宿主结构体前两个字段必须为 `next`, `prev` 指针）。
- `pred`: 插入位置的前驱节点（NULL 表示创建新链表）。

### remque
- `element`: 要移除的节点（必须在链表中）。

**[Post-condition]:**
### insque
- 若 `pred == NULL`: element 成为独立节点（next = prev = NULL）。
- 若 `pred != NULL`: 将 element 插入到 pred 之后，更新 pred->next 和后续节点的 prev。

### remque
- 从链表中摘除 element，更新邻居节点指针。
- element->next 和 element->prev **保持原值不变**（不会被清除）。
- 不释放 element 的内存。

### 不变量

**[Invariant]:** 所有操作基于侵入式双向链表，用户结构体前 2 个指针字段被解释为 `next`, `prev`。

### 意图

POSIX 风格的侵入式双向链表操作。源自 VAX/VMS 系统的 `insque`/`remque` 指令，用于内核和运行时数据结构管理。无需额外内存分配，直接在用户结构体上操作。

### 系统算法

```
insque:
  element->prev = pred
  if (pred):
    element->next = pred->next
    pred->next = element
    if (element->next) element->next->prev = element
  else:
    element->next = NULL

remque:
  if (element->prev) element->prev->next = element->next
  if (element->next) element->next->prev = element->prev
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯指针操作

### 依赖的数据结构
- `struct node`: 内部结构体，包含 `next`, `prev` 两个指针（侵入式链表节点）

### 依赖的外部资源
- `<search.h>`: 函数声明

### 被依赖
- 应用层直接调用（内核和运行时数据结构管理）
