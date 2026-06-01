# seed48 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

unsigned short *seed48(unsigned short seed16v[3]);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seed16v`: 指向 3 个 `unsigned short` 的新种子值。

**[Post-condition]:**
- 设置新种子，重置 mult/add 为默认值。
- 返回指向静态缓冲区的指针，其中存有旧种子值。

### 不变量

**[Invariant]:** 静态缓冲区可能被后续 seed48 调用覆盖。非线程安全。

### 意图

设置 48 位 LCG 种子，同时返回旧种子。便于在自定义种子后恢复之前的 LCG 状态。

### 系统算法

```
保存旧种子至静态 buf; 重置 mult 和 add 为默认 LCG 参数; 设置新种子; 返回旧种子 buf。
```

## 依赖关系

### 依赖的函数
- 直接操作全局变量，无函数调用

### 依赖的数据结构
- `__seed48[7]`: 全局种子数组（读写）
- `__rand48_mult[3]`, `__rand48_add[1]`: 全局 LCG 参数（重置为默认值）
- `static unsigned short old_seed[3]`: 静态缓冲区（存储旧种子值用于返回）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `"rand48.h"`: hidden 全局变量声明

### 被依赖
- 应用层直接调用
