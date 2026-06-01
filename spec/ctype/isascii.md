# isascii 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isascii(int c);
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，任意整数值。

**[Post-condition]:**
- Case 1: `c` 在 0 到 127 范围内（7 位 ASCII）
  - 返回非零值（1）。
- Case 2: `c` 超出 0-127 范围（高位被设置）
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

判断字符是否为 7 位 ASCII 字符。这是 BSD/POSIX 兼容函数，现代代码中很少使用。

### 系统算法

```
return !(c & ~0x7f);
检查 c 的高位（bit 7 及以上）是否全为零。
时间复杂度 O(1)。
```
