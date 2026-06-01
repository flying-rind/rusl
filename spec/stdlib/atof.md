# atof 函数规约

## 复杂度分级: Level 1

---

## 函数接口

```c
#include <stdlib.h>

double atof(const char *s);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`s`: 指向以 null 结尾的字符串。

**[Post-condition]:**
- Case 1 成功转换: 返回 `strtod(s, NULL)` 的值。
- Case 2 无有效数字: 返回 0.0。
- Case 3 溢出: 返回 ±HUGE_VAL。

### 不变量

**[Invariant]:** 纯函数。完全等价于 `strtod(s, 0)`。

### 意图

将字符串转换为 double 类型。仅做一层薄封装，核心逻辑完全委托给 strtod。

### 系统算法

```
return strtod(s, 0);
```

## 依赖关系

### 依赖的函数
- `strtod()`: 核心转换逻辑（stdlib/strtod.c）

### 依赖的数据结构
- 无全局状态

### 依赖的外部资源
- `<stdlib.h>`: 函数声明

### 被依赖
- 应用层直接调用
