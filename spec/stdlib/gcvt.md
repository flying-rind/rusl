# gcvt 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#define _GNU_SOURCE
#include <stdlib.h>
#include <stdio.h>

char *gcvt(double x, int n, char *b);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `x`: 待转换的 double 值。
- `n`: 有效数字位数。
- `b`: 调用者提供的输出缓冲区。

**[Post-condition]:**
- Case 1: 成功将 `x` 格式化为最多 `n` 位有效数字的字符串存入 `b`，返回 `b`。
- 格式规则由 `sprintf(b, "%.*g", n, x)` 的语义定义。

### 不变量

**[Invariant]:** 纯函数。`b` 必须指向足够大的缓冲区。

### 意图

将 double 值转换为 `%g` 格式的字符串，是 ecvt/fcvt 的更高级替代。**此函数已过时，不推荐使用**（非 POSIX 标准）。

### 系统算法

```
sprintf(b, "%.*g", n, x);
return b;
```

## 依赖关系

### 依赖的函数
- `sprintf()`: 格式化浮点数为 %g 格式字符串

### 依赖的数据结构
- `char *b`: 调用者提供的缓冲区（无全局状态）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `<stdio.h>`: sprintf

### 被依赖
- 应用层直接调用（已过时）
