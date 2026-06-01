# islower 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int islower(int c);
int __islower_l(int c, locale_t l);
int islower_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是小写字母（`'a'`-`'z'`）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为小写英文字母。使用无符号区间检查避免分支。

### 系统算法

```
return (unsigned)c-'a' < 26;
通过 unsigned 减法将 'a'-'z' 映射到 0-25，EOF 映射为大值。
时间复杂度 O(1)，无分支。
```
