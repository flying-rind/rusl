# iswxdigit 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <wctype.h>

int iswxdigit(wint_t wc);
int __iswxdigit_l(wint_t c, locale_t l);
int iswxdigit_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是十六进制数字字符（`'0'`-`'9'`、`'A'`-`'F'` 或 `'a'`-`'f'`）
  - 返回非零值。
- Case 2: 其他字符或 `WEOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断宽字符是否为十六进制数字字符。使用两个无符号区间检查：数字区间 `'0'`-`'9'` 和字母区间（通过 `|32` 统一大小写后检查 `'a'`-`'f'`）。

### 系统算法

```
return (unsigned)(wc-'0') < 10 || (unsigned)((wc|32)-'a') < 6;
第一项检查十进制数字，第二项通过 |32 将大写转小写后检查 'a'-'f'。
时间复杂度 O(1)，无分支。
```
