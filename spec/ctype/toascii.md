# toascii 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int toascii(int c);
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，任意整数值。

**[Post-condition]:**
返回 `c & 0x7f`（清除第 7 位及以上所有位），将值映射到 0-127 的 ASCII 范围。

### 不变量

**[Invariant]:** 纯函数。

### 意图

将字符强制转换为 7 位 ASCII。**此函数已过时，不应使用。** 保留仅为 BSD/POSIX 兼容性。

### 系统算法

```
return c & 0x7f;
时间复杂度 O(1)。
```
