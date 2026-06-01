# iscntrl 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int iscntrl(int c);
int __iscntrl_l(int c, locale_t l);
int iscntrl_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是控制字符（0x00-0x1F 或 0x7F）
  - 返回非零值。
- Case 2: 其他字符或 `EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。线程安全。

### 意图

判断字符是否为控制字符。控制字符包括 C0 控制字符（0x00-0x1F）和 DEL（0x7F）。

### 系统算法

```
return (unsigned)c < 0x20 || c == 0x7f;
通过无符号比较同时处理负数（EOF 等）和 0x00-0x1F，再单独检查 DEL。
时间复杂度 O(1)。
```
