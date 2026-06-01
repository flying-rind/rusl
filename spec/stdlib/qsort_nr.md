# qsort_nr 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#define _GNU_SOURCE
#include <stdlib.h>

void __qsort_r(void *base, size_t n, size_t size, int (*cmp)(const void *, const void *, void *), void *arg);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 qsort 的 `base`, `n`, `size`, `cmp`，额外 `arg` 透传给比较函数。

**[Post-condition]:**
- Case 1 (`n > 1`): 委托 `___qsort_r` 执行排序。
- Case 2 (`n <= 1`): 直接返回。

### 不变量

**[Invariant]:** 纯入口函数，不实现排序算法。

### 意图

POSIX `qsort_r` 的实际入口点。仅做参数校验（n > 1 的快速短路），核心逻辑委托给 hidden 符号 `___qsort_r`（位于 qsort.c）。该符号通过 `weak_alias(__qsort_r, qsort_r)` 导出为公共接口。

### 系统算法

```
if (n > 1) ___qsort_r(base, n, size, cmp, arg);
```

## 依赖关系

### 依赖的函数
- `wrapper_cmp()`: 内部 static 函数，将 cmpfun 适配为带 arg 的比较函数签名
- `__qsort_r()`: Smoothsort 排序核心（qsort.c，internal hidden 符号）

### 依赖的数据结构
- 无全局状态

### 依赖的外部资源
- `<stdlib.h>`: 函数声明

### 被依赖
- 应用层直接调用（标准 C qsort 入口）
- 注意：qsort 通过 wrapper_cmp 适配两参数比较函数到三参数 __qsort_r 接口
