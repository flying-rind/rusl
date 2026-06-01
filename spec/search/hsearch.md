# hsearch 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <search.h>

int hcreate(size_t nel);
void hdestroy(void);
ENTRY *hsearch(ENTRY item, ACTION action);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- hcreate: `nel` 为预估的哈希表容量。
- hsearch: `item.key` 非 NULL，`action` 为 FIND 或 ENTER。
- hdestroy 后不可再进行搜索。

**[Post-condition]:**
- hcreate: 成功返回非零，失败返回 0。
- hsearch(FIND): 找到返回 ENTRY*，找不到返回 NULL。
- hsearch(ENTER): 找到返回已有 ENTRY*，找不到则插入并返回新 ENTRY*。失败（内存不足）返回 NULL。

### 不变量

**[Invariant]:** 同一时刻只能存在一个全局哈希表。hdestroy 释放所有内存。非线程安全。

### 意图

POSIX 标准哈希表管理函数。使用全局单一的链式哈希表实现。**此接口已过时**，新的实现建议使用 hcreate_r / hsearch_r 或自定义结构。

### 系统算法

```
hcreate(nel): 分配 buckets 数组（质数 >= nel）
hsearch(item, action):
  - 计算 hash = __gnu_hash(key) % buckets_count
  - 在链表中查找 key (strcmp)
  - FIND: 返回找到的或 NULL
  - ENTER: 找不到则 malloc 新节点插入链表头
```

## 依赖关系

### 依赖的函数
- `keyhash()`: 内部 static 函数，计算字符串哈希值
- `lookup()`: 内部 static 函数，在哈希表中查找 key (二次探测)
- `resize()`: 内部 static 函数，扩容哈希表
- `__hcreate_r()`: 内部 static 函数，可重入版 hcreate（通过 weak_alias 导出为 hcreate_r）
- `__hdestroy_r()`: 内部 static 函数，可重入版 hdestroy
- `__hsearch_r()`: 内部 static 函数，可重入版 hsearch
- `calloc()`, `free()`: 标准库内存管理
- `strcmp()`: 键比较

### 依赖的数据结构
- `struct __tab`: 内部结构体，包含 entries 数组、mask、used 计数
- `static struct hsearch_data htab`: 全局哈希表实例

### 依赖的外部资源
- `<search.h>`: ENTRY, ACTION, hsearch_data 类型定义
- `<stdlib.h>`: calloc/free
- `<string.h>`: strcmp
- weak_alias 宏: 符号弱别名

### 被依赖
- 应用层直接调用（已过时，推荐使用 hcreate_r/hsearch_r/hdestroy_r）
