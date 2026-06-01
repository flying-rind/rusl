# iswdigit 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <wctype.h>

int iswdigit(wint_t wc);
int __iswdigit_l(wint_t c, locale_t l);
int iswdigit_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是十进制数字字符（`L'0'`-`L'9'`）
  - 返回非零值。
- Case 2: 其他字符或 `WEOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断宽字符是否为十进制数字字符。与 `isdigit` 相同的技巧应用于宽字符。

### 系统算法

```
return (unsigned)wc-'0' < 10;
宽字符数字在 BMP 中与 ASCII 数字同值，无符号区间检查同样适用。
时间复杂度 O(1)，无分支。
```
