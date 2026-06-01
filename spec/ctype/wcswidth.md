# wcswidth 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <wchar.h>

int wcswidth(const wchar_t *wcs, size_t n);
```

### 前置/后置条件

**[Pre-condition]:**
- `wcs`: 指向以 null 结尾的宽字符串的指针（为 NULL 时行为未定义）。
- `n`: 最多检查的字符数。

**[Post-condition]:**
- Case 1: 所有 `n` 个字符（或到终止 null）都可打印且列宽已知
  - 返回累计的列宽总和（非负整数）。
- Case 2: 遇到不可打印字符（`wcwidth` 返回 -1）
  - 提前终止，返回 -1。

### 不变量

**[Invariant]:** 纯函数。不修改 `wcs` 指向的内容。

### 意图

计算宽字符串的前 `n` 个字符（或到 null 终止符）的显示列宽总和。若遇到不可打印字符则返回 -1。

### 系统算法

```
遍历 wcs:
  - 当 n > 0 且当前字符非 null 且 wcwidth(*wcs) >= 0:
    l += wcwidth(*wcs), wcs++, n--
  - 若中途 k < 0（遇到不可打印字符），返回 k（即 -1）
  - 正常结束返回 l
时间复杂度 O(n)，每字符调用 wcwidth。
```
