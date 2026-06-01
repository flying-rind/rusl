# isblank 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isblank(int c);
int __isblank_l(int c, locale_t l);
int isblank_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是空格（`' '`）或水平制表符（`'\t'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。不依赖 locale 状态。

### 意图

判断字符是否为空白字符（空格或水平制表符）。C/POSIX locale 下的 `isblank` 语义。

### 系统算法

```
return (c == ' ') || (c == '\t');
时间复杂度 O(1)。
```
