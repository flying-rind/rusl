# isspace 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isspace(int c);
int __isspace_l(int c, locale_t l);
int isspace_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是空白字符（`' '`、`'\t'`、`'\n'`、`'\v'`、`'\f'`、`'\r'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为 C 标准空白字符。使用紧凑的无符号区间技巧：`(unsigned)c-'\t' < 5` 覆盖 `'\t'` 到 `'\r'`（含）五个字符，再单独检查空格。

### 系统算法

```
return c == ' ' || (unsigned)c-'\t' < 5;
独自检查空格，然后用无符号区间检查 '\t'(9) 到 '\r'(13)。
时间复杂度 O(1)。
```
