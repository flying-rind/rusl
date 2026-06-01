# iswspace 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wchar.h>
#include <wctype.h>

int iswspace(wint_t wc);
int __iswspace_l(wint_t c, locale_t l);
int iswspace_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是空白字符
  - `wc` 非零且在预定义的空白字符列表 `spaces[]` 中
  - 返回非零值。
  空白字符列表：`' '`, `'\t'`, `'\n'`, `'\r'`, `'\v'`, `'\f'`, U+0085, U+2000-U+2006, U+2008-U+200A, U+2028, U+2029, U+205F, U+3000
- Case 2: `wc` 不是空白字符或 `wc == 0`
  - `wc == 0` 直接返回 0（`wcschr` 会把 `\0` 误匹配到 `spaces` 的终止符）
  - 其他情况 `wcschr` 未命中返回 0

### 不变量

**[Invariant]:** `spaces[]` 为静态只读常量数组。函数为纯函数。`wc == 0` 特殊处理防止 `wcschr` 误匹配。

### 意图

判断宽字符是否为 Unicode White_Space 属性的空白字符。排除了不间断空格（U+00A0, U+2007, U+202F）和非空白字形的脚本特定字符（U+1680, U+180E）。

### 系统算法

```
if (wc == 0) return 0;  // 防止 wcschr 将 '\0' 匹配到 spaces 数组终止符
return wcschr(spaces, wc) != NULL;
线性扫描 spaces 数组（22个条目），O(n) 但 n 很小且常量。
```
