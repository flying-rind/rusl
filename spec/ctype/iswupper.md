# iswupper 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wctype.h>

int iswupper(wint_t wc);
int __iswupper_l(wint_t c, locale_t l);
int iswupper_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是大写字母（`towlower(wc) != wc`，即存在对应的小写形式）
  - 返回非零值。
- Case 2: `wc` 不是大写字母（无对应小写或 `wc == WEOF`）
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。依赖 `towlower` 的大小写映射表。

### 意图

通过检测 `towlower(wc) != wc` 判断宽字符是否为大写字母。与 `iswlower` 对称，利用大小写转换表反向推断。

### 系统算法

```
return towlower(wc) != wc;
若字符有小写映射（且映射结果不等于自身），则为大写字母。
时间复杂度取决于 towlower 的 casemap 实现，通常为 O(1)。
```
