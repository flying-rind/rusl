# atoi 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <stdlib.h>
#include <ctype.h>

int atoi(const char *s);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
`s`: 指向以 null 结尾的字符串。

**[Post-condition]:**
- Case 1 成功转换: 跳过空白和符号后解析十进制数字，返回对应的 `int` 值。
- Case 2 无有效数字: 返回 0。
- Case 3 溢出: 行为未定义（不设置 errno）。

### 不变量

**[Invariant]:** 累加器 `n` 在循环中始终保持非正值（`n <= 0`），这是安全解析 `int_MIN` 的核心不变量。

### 意图

将十进制字符串转换为 `int` 类型整数。采用**负向累加**策略——中间值始终沿负数方向累加，避免 `int_MIN` 取正溢出导致的未定义行为。

### 系统算法

```
Phase 1: 跳过 isspace 空白字符
Phase 2: switch fall-through 检测符号
Phase 3: 负向累加循环: n = 10*n - (*s++ - '0')
Phase 4: 根据符号返回 n 或 -n
安全证明: 标准正累加在解析 int_MIN 时中间值超出 int_MAX 导致 UB。负向累加全程不产生超出范围的中间值。
```

## 依赖关系

### 依赖的函数
- `isspace()`: 跳过前导空白字符
- `isdigit()`: 判断数字字符

### 依赖的数据结构
- 无全局状态。局部变量 n（负向累加器）和 neg（符号标志）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `<ctype.h>`: isspace/isdigit

### 被依赖
- 应用层直接调用
