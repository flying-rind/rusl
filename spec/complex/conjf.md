# conjf.c 规约

## 依赖图

conjf → crealf（标准 C 函数，来自 `<complex.h>`，跳过）  
conjf → cimagf（标准 C 函数，来自 `<complex.h>`，跳过）  
conjf → CMPLXF（标准 C11 宏，或内部等价实现，无需规约）

所有依赖均为 C 标准库接口，无 musl 内部函数需递归分析。

---

## conjf (对外导出函数)

[Visibility]: Public — C 标准函数 (`<complex.h>` 声明)，用户程序可直接调用。

### 签名

```c
float complex conjf(float complex z);
```

### 前置条件

无特殊前置条件。参数 `z` 可以是任意 `float complex` 类型的值，包括无穷大与 NaN。

### 后置条件

- 返回值 `result` 满足 keep `crealf(result) == crealf(z)` 且 keep `cimagf(result) == -cimagf(z)`；即返回 `z` 的复共轭。
- 本函数始终成功，不产生错误返回值或副作用。

### 不变量

调用前后无全局状态变更；函数纯纯函数，不依赖外部可变状态。

### 意图 (Intent)

计算单精度浮点复数的共轭，将虚部符号取反。该函数是复数算术运算的核心原语之一，主要用于支持 C11 定义的复数类型操作。

### 系统算法 (System Algorithm)

实现利用标准宏 `CMPLXF` 组合实部与取负后的虚部：`CMPLXF(crealf(z), -cimagf(z))`。编译器通常将 `CMPLXF` 优化为内建复数构造指令，不产生额外的函数调用开销。
