# tolower 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int tolower(int c);
int __tolower_l(int c, locale_t l);
int tolower_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是大写字母（`'A'`-`'Z'`）
  - 返回对应的小写字母（`c | 32`，即 `c + 32`）。
- Case 2: `c` 不是大写字母
  - 返回原值 `c`。

### 不变量

**[Invariant]:** 纯函数。线程安全。

### 意图

将大写字母转换为小写字母。使用 `c | 32` 技巧（ASCII 中大写字母 bit5 为 0，小写为 1）实现高效转换。

### 系统算法

```
if (isupper(c)) return c | 32;
return c;
先通过 isupper 判断，若为大写则设置 bit5（| 32）完成转换。
时间复杂度 O(1)。
```
