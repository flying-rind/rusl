# lsearch 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <search.h>

void *lsearch(const void *key, void *base, size_t *nelp, size_t width, int (*compar)(const void *, const void *));
void *lfind(const void *key, const void *base, size_t *nelp, size_t width, int (*compar)(const void *, const void *));
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `key`: 查找键。
- `base`: 数组基地址。
- `nelp`: 元素个数指针。
- `width`: 元素大小。
- `compar`: 比较函数。

**[Post-condition]:**
- lsearch: 找到返回匹配元素指针；未找到则追加到数组末尾，`*nelp` 加 1，返回新元素指针。
- lfind: 找到返回匹配元素指针；未找到返回 NULL，不修改数组。

### 不变量

**[Invariant]:** lsearch 修改数组内容（追加新元素）。lfind 只读。

### 意图

在无序数组中执行线性搜索。lsearch 找不到时自动追加（类似惰性去重集合）。POSIX 标准提供的简单查找工具。

### 系统算法

```
for (i = 0; i < *nelp; i++)
  if (compar(key, base + i*width) == 0) return base + i*width
// lsearch: memcpy(base + *nelp*width, key, width); (*nelp)++; return base + (*nelp-1)*width
// lfind: return NULL
```

## 依赖关系

### 依赖的函数
- `compar()`: 调用者提供的比较函数（函数指针参数）
- `memcpy()`: 标准库内存复制（lsearch 插入时使用）

### 依赖的数据结构
- 无全局状态。所有数据由调用者通过参数提供。

### 依赖的外部资源
- `<search.h>`: 函数声明
- `<string.h>`: memcpy

### 被依赖
- 应用层直接调用
