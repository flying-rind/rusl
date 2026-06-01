# imaxabs 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <inttypes.h>

intmax_t imaxabs(intmax_t a);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`a`: 类型为 `intmax_t`，任意合法值（除 `intmax_t_MIN` 外，其绝对值不可表示）。

**[Post-condition]:**
- Case 1: `a >= 0` → 返回 `a`。
- Case 2: `a < 0` → 返回 `-a`。
- 若 `a == intmax_t_MIN`，行为未定义。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

返回整数 `a` 的绝对值（intmax_t 类型）。

### 系统算法

```
return a > 0 ? a : -a;
时间复杂度 O(1)。
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯算术运算

### 依赖的数据结构
- 无全局状态

### 依赖的外部资源
- `<inttypes.h>`: intmax_t 类型定义、函数声明

### 被依赖
- 应用层直接调用
