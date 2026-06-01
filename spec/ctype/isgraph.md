# isgraph 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isgraph(int c);
int __isgraph_l(int c, locale_t l);
int isgraph_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是可打印图形字符（0x21-0x7E，即除空格外的可打印 ASCII 字符）
  - 返回非零值。
- Case 2: 其他字符（包括空格 0x20）或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为图形字符（有可见字形的可打印字符，排除空格）。

### 系统算法

```
return (unsigned)c-0x21 < 0x5e;
通过无符号区间检查，0x21 到 0x7E（含）映射到 0 到 0x5D。
时间复杂度 O(1)，无分支。
```
