# rand 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <stdlib.h>

int rand(void);
void srand(unsigned int seed);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `rand`: 无参数，依赖全局种子。
- `srand`: `seed` 为任意 `unsigned int` 值，用于初始化全局种子。

**[Post-condition]:**
- `rand()`: 返回 [0, RAND_MAX] 的伪随机整数。
- `srand(seed)`: 设置全局种子，后续 rand() 序列由 seed 确定。

### 不变量

**[Invariant]:** 全局种子 `__rand_seed` 在多线程下无锁保护，存在数据竞争。

### 意图

标准 C 库的简单伪随机数生成器。使用经典 LCG（乘数 1103515245，加数 12345，模数 2^31）。srand 用于设置种子。

### 系统算法

```
rand(): __rand_seed = __rand_seed * 1103515245 + 12345; return (__rand_seed >> 16) & RAND_MAX;
srand(seed): __rand_seed = seed
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯算术运算

### 依赖的数据结构
- `static uint64_t seed`: 文件内静态全局种子变量

### 依赖的外部资源
- `<stdlib.h>`: RAND_MAX 宏、函数声明
- `<stdint.h>`: uint64_t 类型

### 被依赖
- 应用层直接调用
- 注意：与 rand_r 共享相同的 LCG 公式但使用独立状态
