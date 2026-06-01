# rand_r 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

int rand_r(unsigned int *seed);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`seed`: 指向调用者维护的 `unsigned int` 种子变量。

**[Post-condition]:**
- `*seed` 被更新为 `*seed * 1103515245 + 12345`。
- 返回 `(*seed_new) & RAND_MAX`（低 31 位非负值）。

### 不变量

**[Invariant]:** 纯函数。所有状态由调用者通过 `seed` 参数管理，无全局状态。

### 意图

可重入版本的 `rand()`。使用与 `rand()` 完全相同的 LCG 参数，但状态由调用者显式管理，天然线程安全。

### 系统算法

```
*seed = *seed * 1103515245 + 12345;
return *seed & RAND_MAX;
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯算术运算

### 依赖的数据结构
- `unsigned int *seed`: 调用者管理的种子（通过参数传入，无全局状态）

### 依赖的外部资源
- `<stdlib.h>`: RAND_MAX 宏、函数声明

### 被依赖
- 应用层直接调用（线程安全版本）
