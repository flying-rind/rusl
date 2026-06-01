# qsort 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef int (*cmpfun)(const void *, const void *, void *);

void qsort(void *base, size_t nel, size_t width, cmpfun cmp);
void __qsort_r(void *base, size_t nel, size_t width, cmpfun cmp, void *arg);
void ___qsort_r(void *base, size_t nel, size_t width, cmpfun cmp, void *arg);  // hidden
```

### 前置/后置条件

**[Visibility]:** Public — 对外导出 (C 标准函数)

**[Pre-condition]:**
### qsort
- `base`: 指向待排序数组。`nel * width` 大小的可读写内存。
- `nel`: 元素个数。
- `width`: 每个元素的字节大小。
- `cmp`: 比较函数指针，返回 `<0`, `0`, `>0`。不得修改元素。

### __qsort_r / qsort_r (POSIX 扩展)
- 额外 `arg` 参数透传给 `cmp`，实现可重入排序。

**[Post-condition]:**
- Case 1 (`nel > 1`): 数组按 `cmp` 升序完成原地排序。
- Case 2 (`nel <= 1`): 数组不变。
- 排序是**不稳定**的（等价元素相对顺序不保证）。
- 内存安全：不会越界写入 `[base, base + nel*width)` 范围。
- qsort_r 线安全（无全局状态），qsort 通过 `__qsort_r` + 全局函数指针间接调用。

### 不变量

**[Invariant]:** - Leonardo 堆不变量：排序过程中维护一组 Leonardo 堆，保证堆大小均为 Leonardo 数。
- 排列不变量：排序前后元素集合同构，仅顺序改变。
- 无递归：使用显式栈管理堆，避免栈溢出。

### 意图

实现 Smoothsort（平滑排序），Heapsort 的自适应变体：
- 时间复杂度: 最坏 O(n log n)，几乎有序时接近 O(n)
- 空间复杂度: O(1)（原地排序）
- 核心结构: Leonardo 堆 + 双字位运算 (ntz, shl, shr)
- qsort 是 qsort_r 的简单包装，通过运行时函数指针间接调用 cmp

### 系统算法

```
Phase 1 (构建): 遍历数组逐个元素，构建 Leonardo 堆森林，维护堆序。
Phase 2 (整理): 从后向前取出最大元素，通过 trinkle 恢复堆序。
Phase 3 (归位): 每次取出后将最大元素放至数组末尾。
子过程:
  - sift: 筛选操作（下沉/上浮），维护堆性质
  - trinkle: 跨堆合并（当堆大小序列需要时）
  - cycle: 循环置换辅助函数
  - shl/shr: 双字位移（用于 Leonardo 数运算）
  - pntz: 双字位尾零计数
```

## 依赖关系

### 依赖的函数
- `sift()`: 内部 static 函数，堆筛选（下沉/上浮）操作
- `trinkle()`: 内部 static 函数，跨 Leonardo 堆合并
- `cycle()`: 内部 static 函数，循环置换辅助（使用 memcpy）
- `shl()`: 内部 static inline 函数，双字左移
- `shr()`: 内部 static inline 函数，双字右移
- `pntz()`: 内部 static inline 函数，双字位尾零计数
- `a_ctz_l()`: musl 原子操作库的 CTZ 原语（通过 atomic.h 引入，宏 ntz 的底层实现）
- `memcpy()`: cycle 函数中的内存块移动

### 依赖的数据结构
- `size_t lp[]`: Leonardo 数表（预计算，以元素宽度缩放）
- `size_t p[2]`: 双字位向量（表示 Leonardo 堆森林结构）
- `unsigned char *ar[]`: 循环置换工作数组（AR_LEN 个元素）

### 依赖的外部资源
- `<stdint.h>`: size_t 类型（通过 stdlib.h 间接）
- `<stdlib.h>`: size_t、NULL、函数声明
- `<string.h>`: memcpy
- `"atomic.h"`: a_ctz_l 原子操作原语

### 被依赖
- qsort: 标准 C 排序函数入口（qsort_nr.c 中的包装）
- __qsort_r: POSIX qsort_r 的入口
