# iswgraph 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <wctype.h>

int iswgraph(wint_t wc);
int __iswgraph_l(wint_t c, locale_t l);
int iswgraph_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是可打印且非空格的宽字符（`iswprint(wc) && !iswspace(wc)`）
  - 返回非零值。
- Case 2: 其他字符或 `WEOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。组合 `iswprint` 和 `iswspace` 实现。

### 意图

判断宽字符是否为图形字符。按 ISO C 定义等价于 `!iswspace(wc) && iswprint(wc)`。

### 系统算法

```
return !iswspace(wc) && iswprint(wc);
先检查 iswspace（更便宜的操作），短路后可避免 iswprint 的位图查找。
```
