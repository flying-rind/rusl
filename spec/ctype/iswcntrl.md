# iswcntrl 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wctype.h>

int iswcntrl(wint_t wc);
int __iswcntrl_l(wint_t c, locale_t l);
int iswcntrl_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是控制字符，满足以下任一条件:
  - `wc < 32`（C0 控制字符）
  - `wc` 在 `[0x7F, 0x9F]` 范围内（DEL + C1 控制字符）
  - `wc` 在 `[0x2028, 0x2029]` 范围内（行/段分隔符）
  - `wc` 在 `[0xFFF9, 0xFFFB]` 范围内（行间注释锚点）
  - 返回非零值。
- Case 2: 其他字符
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。线程安全。

### 意图

判断宽字符是否为 Unicode 控制字符。覆盖 C0、C1、Unicode 行分隔符和特殊控制字符。

### 系统算法

```
return (unsigned)wc < 32
    || (unsigned)(wc-0x7f) < 33     // 0x7F-0x9F
    || (unsigned)(wc-0x2028) < 2    // 0x2028-0x2029
    || (unsigned)(wc-0xfff9) < 3;   // 0xFFF9-0xFFFB
使用四个无符号区间检查，O(1) 时间复杂度。
```
