# iswblank 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <wctype.h>
#include <ctype.h>

int iswblank(wint_t wc);
int __iswblank_l(wint_t c, locale_t l);
int iswblank_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是空白字符（空格 `L' '` 或水平制表符 `L'\t'`）
  - 返回非零值（委托给 `isblank(wc)`）。
- Case 2: 其他字符或 `WEOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。直接委托给 `isblank`。

### 意图

判断宽字符是否为空白字符（空格或水平制表符）。由于 ASCII 空格和制表符在宽字符中值与 `char` 相同，直接委托给 `isblank`。

### 系统算法

```
return isblank(wc);
由于 ' ' 和 '\t' 的 wint_t 值等价于其 char 值，直接复用 isblank。
时间复杂度 O(1)。
```
