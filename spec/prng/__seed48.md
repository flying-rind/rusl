# __seed48 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <stdint.h>

uint64_t __seed48(unsigned short *s);  // hidden
```

### 前置/后置条件

**[Visibility]:** Internal — musl 内部实现，hidden 符号不对外导出

**[Pre-condition]:**
`s`: 指向 3 个 `unsigned short` 的数组（48 位种子值）。

**[Post-condition]:**
- 将 `s` 的值存入全局 `__seed48`（实际为全局 TLS 状态的一部分）。
- 返回构建的 48 位种子值。该值同时被写入 `__seed48` 全局。

### 不变量

**[Invariant]:** 修改全局状态，非线程安全。

### 意图

将 48 位值写入全局种子变量，供 `__rand48_step` 读取。**内部 hidden 符号，不对外导出**。

### 系统算法

```
从 s[0..2] 拼装 48 位值，写入全局 __seed48 (TLS)，返回该值。
```

## 依赖关系

### 依赖的函数
- 无外部函数依赖

### 依赖的数据结构
- `unsigned short __seed48[7]`: 全局种子数组（定义于 __seed48.c），实际为 TLS 存储

### 依赖的外部资源
- `"rand48.h"`: hidden 全局变量声明

### 被依赖
- 所有 drand48 族函数通过 `__rand48_step` 间接读取 `__seed48`
- seed48 函数间接更新 `__seed48`
