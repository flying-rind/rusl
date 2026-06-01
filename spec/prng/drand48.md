# drand48 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <stdlib.h>

double drand48(void);
double erand48(unsigned short xsubi[3]);
long lrand48(void);
long nrand48(unsigned short xsubi[3]);
long mrand48(void);
long jrand48(unsigned short xsubi[3]);
void srand48(long seedval);
unsigned short *seed48(unsigned short seed16v[3]);
void lcong48(unsigned short p[7]);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
各函数的前置条件因其操作对象不同：一部分使用全局 TLS 状态 `__rand48_dt`，另一部分使用调用者传入的 `xsubi` / `seed16v` / `p` 缓冲区。

**[Post-condition]:**
所有生成函数基于 48 位 LCG: X_{n+1} = (a * X_n + c) mod 2^48
- drand48: 返回 [0.0, 1.0) 的 double (高 48 位 / 2^48)
- lrand48: 返回 [0, 2^31) 的 long (高 31 位)
- mrand48: 返回 [-2^31, 2^31) 的 long (高 32 位含符号)
- erand48/nrand48/jrand48: 使用调用者提供的种子状态

### 不变量

**[Invariant]:** 全局状态 `__rand48_dt` 在多线程访问下不设锁保护。erand48/nrand48/jrand48 的种子参数为调用者内存，无全局竞争。

### 意图

实现 SUSv2/POSIX 48 位线性同余生成器族。状态由 7 个 `unsigned short` 构成：
{seed[0..2], mult[0..2], add[0]}。默认参数 a=0x5DEECE66D, c=0xB。
核心迭代委托给 hidden 函数 `__rand48_step`。

### 系统算法

```
drand48(): s = __rand48_step(__seed48, __rand48_dt.__mult, __rand48_dt.__add), return s / 2^48
lrand48(): s = __rand48_step(...), return (s >> 17) & 0x7FFFFFFF
mrand48(): s = __rand48_step(...), return (int32_t)(s >> 16)
srand48(seed): 设 mult=default, add=default, seed = {0x330E, seed & 0xFFFF, seed >> 16}
```

## 依赖关系

### 依赖的函数
- `__rand48_step()`: LCG 单步迭代核心（internal, prng/__rand48_step.c）
- `__seed48`: 全局种子数组（internal, prng/__seed48.c）

### 依赖的数据结构
- `__rand48_dt`: TLS 全局结构体，包含 mult[3], add[1], seed[3]

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `<stdint.h>`: uint64_t 类型
- `"rand48.h"`: hidden 符号声明

### 被依赖
- 应用层直接调用
