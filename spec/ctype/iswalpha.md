# iswalpha 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <wctype.h>

static const unsigned char table[] = {
#include "alpha.h"
};

int iswalpha(wint_t wc);
int __iswalpha_l(wint_t c, locale_t l);
int iswalpha_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是 Unicode 字母字符
  - `wc < 0x20000U` 且二级位图查找命中：返回 1
  - `wc` 在 `[0x20000U, 0x2fffeU)` 范围内：返回 1
- Case 2: `wc` 不是字母字符
  - `wc >= 0x2fffeU`：返回 0
  - `wc < 0x20000U` 但位图查找未命中：返回 0

### 不变量

**[Invariant]:** - `table` 静态数组（来自 `alpha.h`）为只读常量，程序生命周期内不变。
- 函数为纯函数，无副作用，线程安全。

### 意图

判断宽字符是否为 Unicode 字母字符。使用二级位图查找表覆盖 BMP 及 Supplementary Multilingual Plane（到 U+1FFFF）的所有码点，对 CJK Extension B 范围（U+20000-U+2FFFD）使用硬编码返回真。

### 系统算法

```
Phase 1（快速路径-BMP）:
  wc < 0x20000 时，使用二级位图查找:
    - table[wc>>8]: 获取高 8 位对应的二级表偏移
    - 索引 = table[高位] * 32 + ((wc & 255) >> 3)
    - 位掩码 = 1 << (wc & 7)
    - 返回位图命中结果

Phase 2（CJK Extension B）:
  0x20000 <= wc < 0x2fffe 时直接返回 1

Phase 3（越界）:
  wc >= 0x2fffe 返回 0

时间复杂度 O(1)，使用编译时生成的位图表。
```
