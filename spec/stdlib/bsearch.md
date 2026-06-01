# bsearch 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

void *bsearch(const void *key, const void *base, size_t nel, size_t width, int (*cmp)(const void *, const void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `key`: 指向要查找的元素的指针。
- `base`: 指向已排序数组的指针。
- `nel`: 元素个数。
- `width`: 每个元素的字节大小。
- `cmp`: 比较函数，返回 `<0`, `0`, `>0`。
- 数组必须已按 `cmp` 升序排列。

**[Post-condition]:**
- Case 1 找到: 返回指向匹配元素的指针。
- Case 2 未找到: 返回 NULL。
- 若多个元素匹配，返回任意一个（C 标准不指定哪个）。

### 不变量

**[Invariant]:** 纯函数。不修改数组和 key 内容。

### 意图

在已排序数组中二分查找指定元素。标准 C 库函数。

### 系统算法

```
while (nel > 0):
  try = base + width * (nel / 2)
  sign = cmp(key, try)
  if sign < 0: nel /= 2
  elif sign > 0: base = try + width; nel -= nel/2 + 1
  else: return try
return NULL
时间复杂度 O(log n)。
```

## 依赖关系

### 依赖的函数
- `cmp()`: 调用者提供的比较函数（函数指针参数）

### 依赖的数据结构
- 无全局状态。通过指针运算在调用者提供的数组上操作。

### 依赖的外部资源
- `<stdlib.h>`: size_t 类型、函数声明

### 被依赖
- 应用层直接调用
