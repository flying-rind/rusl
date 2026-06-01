# lrand48 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

long lrand48(void);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
无参数。使用全局 TLS 状态 `__rand48_dt`。

**[Post-condition]:**
推进全局 LCG 一步，返回 [0, 2^31) 的非负 long 值（高 31 位）。

### 不变量

**[Invariant]:** 读取并修改全局状态，非线程安全。

### 意图

返回非负伪随机长整数的 48 位 LCG 生成器。见 drand48.c 的详细规约。

### 系统算法

```
return (long)(__rand48_step(__seed48, __rand48_mult, __rand48_add) >> 17) & 0x7FFFFFFF;
```

## 依赖关系

### 依赖的函数
- `__rand48_step()`: LCG 迭代核心（internal）

### 依赖的数据结构
- `__seed48[7]`: 全局种子数组
- `__rand48_mult[3]`, `__rand48_add[1]`: 全局 LCG 参数

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `"rand48.h"`: hidden 符号声明

### 被依赖
- 应用层直接调用
