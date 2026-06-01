# isalpha 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <ctype.h>

int isalpha(int c);
int __isalpha_l(int c, locale_t l);
int isalpha_l(int c, locale_t l);  // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
`c`: 类型为 `int`，值必须可表示为 `unsigned char` 或等于 `EOF`。

**[Post-condition]:**
- Case 1: `c` 是字母（`'a'`-`'z'` 或 `'A'`-`'Z'`）
  - 返回非零值。
- Case 2: `c` 不是字母，或 `c == EOF`
  - 返回 0。

### 不变量

**[Invariant]:** 纯函数。不依赖 locale（此实现忽略 locale 参数）。

### 意图

判断字符是否为英文字母。使用位运算技巧 `((unsigned)c|32)-'a' < 26` — 将大写转为小写后统一比较，消除分支。

### 系统算法

```
将 c 转为 unsigned 类型，通过 |32 将大写字母转为小写，
然后判断是否在 'a' 到 'z' 范围内（差小于 26）。
时间复杂度 O(1)，无分支。
```
