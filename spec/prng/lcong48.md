# lcong48 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

void lcong48(unsigned short p[7]);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`p`: 指向 7 个 `unsigned short` 的数组，其中 p[0..2] 为新种子，p[3..5] 为新乘数，p[6] 为新加数。

**[Post-condition]:**
设置全局 `__rand48_dt` 的 seed, mult, add 为 `p` 指定的值。后续 drand48/lrand48/mrand48 的序列由新参数确定。

### 不变量

**[Invariant]:** 直接修改全局状态，非线程安全。

### 意图

一次性设置 48 位 LCG 的全部参数（种子、乘数、加数）。允许用户完全自定义 LCG 参数。

### 系统算法

```
__rand48_dt.__seed[i] = p[i]; __rand48_dt.__mult[i] = p[i+3]; __rand48_dt.__add = p[6];
```

## 依赖关系

### 依赖的函数
- 直接操作全局变量，无函数调用

### 依赖的数据结构
- `__rand48_dt`: TLS 全局结构体，包含 seed[3], mult[3], add[1]

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `"rand48.h"`: hidden 全局变量声明

### 被依赖
- 应用层直接调用
