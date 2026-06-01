# isprint 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isprint(int c);
int __isprint_l(int c, locale_t l);
int isprint_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是可打印字符（0x20-0x7E，包含空格）
  - 返回非零值。
- Case 2: 其他字符（控制字符、DEL 等）或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为可打印字符（含空格，即 `isgraph(c) || c == ' '`）。

### 系统算法

```
return (unsigned)c-0x20 < 0x5f;
通过无符号区间检查，0x20 到 0x7E（含）映射到 0 到 0x5E。
时间复杂度 O(1)，无分支。
```
