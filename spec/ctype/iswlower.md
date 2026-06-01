# iswlower 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wctype.h>

int iswlower(wint_t wc);
int __iswlower_l(wint_t c, locale_t l);
int iswlower_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是小写字母（`towupper(wc) != wc`，即存在对应的大写形式）
  - 返回非零值。
- Case 2: `wc` 不是小写字母（无对应大写或 `wc == WEOF`）
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。依赖 `towupper` 的大小写映射表。

### 意图

通过检测 `towupper(wc) != wc` 判断宽字符是否为小写字母。避免了维护独立的小写字母分类表，利用大小写转换表反向推断。

### 系统算法

```
return towupper(wc) != wc;
若字符有大写映射（且映射结果不等于自身），则为小写字母。
时间复杂度取决于 towupper 的 casemap 实现，通常为 O(1)。
```
