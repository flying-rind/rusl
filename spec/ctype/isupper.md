# isupper 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isupper(int c);
int __isupper_l(int c, locale_t l);
int isupper_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是大写字母（`'A'`-`'Z'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为大写英文字母。与 `islower` 对称实现。

### 系统算法

```
return (unsigned)c-'A' < 26;
通过 unsigned 减法将 'A'-'Z' 映射到 0-25，EOF 映射为大值。
时间复杂度 O(1)，无分支。
```
