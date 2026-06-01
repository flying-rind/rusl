# fcvt 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#define _GNU_SOURCE
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

char *fcvt(double x, int n, int *dp, int *sign);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `x`: 待转换的 double 值。
- `n`: 小数位数（最大 1400）。
- `dp`: 输出小数点的位置。
- `sign`: 输出符号（0 正 1 负）。

**[Post-condition]:**
- 返回指向字符串的指针，表示 `x` 的定点格式小数部分。
- `*dp` 指示小数点位置。
- 若前导零过多，返回固定字符串 "000000000000000"。
- 否则委托给 `ecvt` 完成转换。

### 不变量

**[Invariant]:** 大型格式化可能使用 ecvt 的静态缓冲区。

### 意图

将 double 值转换为定点格式的字符串表示。**此函数已过时**。核心技巧是计算前导零数量 (`lz`) 后委托给 ecvt。

### 系统算法

```
sprintf(tmp, "%.*f", n, x);
计算前导零数量 lz (strspn / strcspn)
若 n <= lz 则返回 "000..." 字符串
否则调用 ecvt(x, n-lz, dp, sign)
```

## 依赖关系

### 依赖的函数
- `sprintf()`: 格式化浮点数为定点字符串（%.*f）
- `strspn()`: 计算前导零数量
- `strcspn()`: 查找小数点位置
- `ecvt()`: 委托完成最终转换（计算 lz 后）

### 依赖的数据结构
- `char tmp[1500]`: 栈上临时缓冲区
- 返回值可能指向 ecvt 的静态缓冲区或常量字符串

### 依赖的外部资源
- `<stdlib.h>`: ecvt
- `<stdio.h>`: sprintf
- `<string.h>`: strspn/strcspn

### 被依赖
- 应用层直接调用（已过时）
