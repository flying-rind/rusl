# __rand48_step 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <stdint.h>

uint64_t __rand48_step(unsigned short *xi, unsigned short *lc, unsigned short *state);  // hidden
```

### 前置/后置条件

**[Visibility]:** Internal — musl 内部实现，hidden 符号不对外导出

**[Pre-condition]:**
- `xi`: 当前 48 位种子（3 个 `unsigned short` 小端排列）。
- `lc`: 乘数（3 个）和加数（1 个），共 4 个 `unsigned short`。
- `state`: 输出新状态（3 个 `unsigned short`），可与 `xi` 同址。

**[Post-condition]:**
- 执行 X_new = (a * X_curr + c) mod 2^48。
- `state` 接收新的 48 位值（小端排列）。
- 返回 X_new（低 48 位）。

### 不变量

**[Invariant]:** 确定性纯函数。给定相同 `xi` 和 `lc` 产生相同输出。

### 意图

48 位 LCG 单步迭代核心引擎。所有 drand48 族函数的底层算术实现。**内部 hidden 符号，不对外导出**。

### 系统算法

```
在 64 位无符号算术中计算 a * X_curr + c，结果自动截断至 48 位。
代码中使用 +0U / +0ULL 显式类型提升防止 16-bit 平台移位溢出。
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯算术运算

### 依赖的数据结构
- `unsigned short *xi`: 调用者传入的 48 位种子（3 个 unsigned short）
- `unsigned short *lc`: 调用者传入的乘数+加数（4 个 unsigned short）

### 依赖的外部资源
- `<stdint.h>`: uint64_t 类型
- `"rand48.h"`: hidden 函数声明

### 被依赖
- drand48.c 中的 drand48/erand48/lrand48/nrand48/mrand48/jrand48
- seed48/__seed48 设置种子后通过此函数推进 LCG
