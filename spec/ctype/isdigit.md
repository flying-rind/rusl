# isdigit 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isdigit(int c);
int __isdigit_l(int c, locale_t l);
int isdigit_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是十进制数字（`'0'`-`'9'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为十进制数字字符。使用经典的无符号区间检查技巧。

### 系统算法

```
return (unsigned)c-'0' < 10;
通过 unsigned 减法将 '0'-'9' 映射到 0-9，同时将 EOF（-1）映射为大值（>= 10）。
时间复杂度 O(1)，无分支。
```
