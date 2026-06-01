# ecvt 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#define _GNU_SOURCE
#include <stdlib.h>
#include <stdio.h>

char *ecvt(double x, int n, int *dp, int *sign);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `x`: 待转换的 double 值。
- `n`: 有效数字位数（最大 15）。
- `dp`: 输出小数点的位置。
- `sign`: 输出符号（0 正 1 负）。

**[Post-condition]:**
- 返回指向静态缓冲区的指针，包含 `x` 的 `n` 位有效数字（无小数点）。
- `*dp` 设置为小数点相对于返回字符串首位的位置。
- `*sign` 设置为 0（正）或 1（负）。

### 不变量

**[Invariant]:** 使用静态缓冲区 `buf[16]`，**非线程安全**。多次调用会覆盖之前结果。

### 意图

将 double 值转换为科学计数法风格的字符串表示。**此函数已过时，不推荐使用**，应使用 `sprintf` 或 `snprintf`。

### 系统算法

```
sprintf(tmp, "%.*e", n-1, x);
解析临时字符串，跳过符号位，拷贝有效数字至静态 buf，
用 atoi 解析指数部分计算小数点位。
```

## 依赖关系

### 依赖的函数
- `sprintf()`: 格式化浮点数为科学计数法字符串
- `atoi()`: 解析指数部分计算小数点位

### 依赖的数据结构
- `static char buf[16]`: 静态缓冲区（非线程安全，后续调用会覆盖）

### 依赖的外部资源
- `<stdlib.h>`: atoi
- `<stdio.h>`: sprintf

### 被依赖
- fcvt: 委托调用（计算前导零后）
