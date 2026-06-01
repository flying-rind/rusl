# isxdigit 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isxdigit(int c);
int __isxdigit_l(int c, locale_t l);
int isxdigit_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是十六进制数字字符（`'0'`-`'9'`、`'A'`-`'F'` 或 `'a'`-`'f'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为十六进制数字字符。复用 `isdigit` 检测数字，用 `|32` 技巧统一大小写后检查字母范围。

### 系统算法

```
return isdigit(c) || ((unsigned)c|32)-'a' < 6;
先委派给 isdigit，若失败则通过 |32 将大写转小写后检查 'a'-'f'。
时间复杂度 O(1)。
```
