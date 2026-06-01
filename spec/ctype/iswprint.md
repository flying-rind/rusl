# iswprint 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <wctype.h>

int iswprint(wint_t wc);
int __iswprint_l(wint_t c, locale_t l);
int iswprint_l(wint_t c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1: `wc` 是可打印字符
  - 返回非零值。
  判定逻辑：
  - `wc < 0xff` 且 `(wc+1 & 0x7f) >= 0x21`（低7位在 [0x20, 0x7E]，即 ASCII 可打印）
  - 或 `wc < 0x2028`（BMP 且非控制字符）
  - 或 `wc` 在 `[0x202A, 0xD7FF]` 范围内（排除行分隔符 0x2028-0x2029）
  - 或 `wc` 在 `[0xE000, 0xFFF8]` 范围内（私用区等）
- Case 2: `wc` 不是可打印字符
  - `wc >= 0xfffc` 或 `(wc & 0xfffe) == 0xfffe`（非字符码点）
  - 或落入上述排除区间
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。线程安全。

### 意图

判断宽字符是否为可打印字符。排除 C0/C1 控制字符、行/段分隔符（U+2028-U+2029）、行间注释锚点（U+FFF9-U+FFFB）和非字符码点（U+FFFE、U+FFFF 等）。针对常见可打印字符热路径进行了高度优化。

### 系统算法

```
Phase 1: wc < 0xFF → 使用位运算检查低7位是否在 [0x20, 0x7E]
Phase 2: wc < 0x2028 → 直接返回真（BMP 中的可打印区）
Phase 3: wc 在 0x202A..0xD7FF 或 0xE000..0xFFF8 → 返回真
Phase 4: wc >= 0xFFFC → 返回假（排除非字符和越界码点）
         或 (wc & 0xFFFE) == 0xFFFE → 返回假（非字符码点）
Phase 5: 其余返回真（CJK 等高位平面字符）
```
