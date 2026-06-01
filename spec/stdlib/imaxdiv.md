# imaxdiv 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <inttypes.h>

imaxdiv_t imaxdiv(intmax_t num, intmax_t den);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `num`: 被除数。
- `den`: 除数，**必须非零**。
- `num == intmax_t_MIN && den == -1` 时行为未定义。

**[Post-condition]:**
- Case 1 正常: 返回 `imaxdiv_t` 结构体，其中 `quot = num / den`（向零截断），`rem = num % den`，满足 `num == quot * den + rem`。

### 不变量

**[Invariant]:** 纯函数。无内部状态。

### 意图

对 `intmax_t` 类型执行除法，同时返回商和余数。语义与 C 内建 `/` 和 `%` 运算符完全一致。

### 系统算法

```
return (imaxdiv_t){ num/den, num%den };
编译器通常生成单条除法指令同时获取商和余数。
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖，纯算术运算

### 依赖的数据结构
- 无全局状态

### 依赖的外部资源
- `<inttypes.h>`: intmax_t/imaxdiv_t 类型定义、函数声明

### 被依赖
- 应用层直接调用
