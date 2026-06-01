# ispunct 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int ispunct(int c);
int __ispunct_l(int c, locale_t l);
int ispunct_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是标点符号（可打印但非字母数字、非空格）
  - 即 `isgraph(c) && !isalnum(c)` 为真时返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为标点符号。标点符号定义为可打印图形字符中排除字母和数字的部分。

### 系统算法

```
return isgraph(c) && !isalnum(c);
组合 isgraph 和 isalnum 实现。
时间复杂度 O(1)。
```
