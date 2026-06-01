# srand48 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

void srand48(long seedval);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seedval`: 任意 `long` 值，仅低 32 位用于设置种子。

**[Post-condition]:**
- 重置 mult 和 add 为默认 LCG 参数。
- 设置新种子为 {0x330E, seedval & 0xFFFF, (seedval >> 16) & 0xFFFF}。

### 不变量

**[Invariant]:** 修改全局状态，非线程安全。

### 意图

通过 32 位种子值初始化 48 位 LCG。高 16 位固定为 0x330E。

### 系统算法

```
__rand48_mult = default_mult; __rand48_add = default_add;
__seed48 = {0x330E, seedval & 0xFFFF, (seedval >> 16) & 0xFFFF};
```

## 依赖关系

### 依赖的函数
- 无函数调用，直接操作全局变量

### 依赖的数据结构
- `__seed48[7]`: 全局种子数组（写入）
- `__rand48_mult[3]`, `__rand48_add[1]`: 全局 LCG 参数（重置为默认值）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `"rand48.h"`: hidden 全局变量声明

### 被依赖
- 应用层直接调用
