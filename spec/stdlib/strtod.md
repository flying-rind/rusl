# strtod 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <stdlib.h>

double strtod(const char *restrict s, char **restrict endptr);
float strtof(const char *restrict s, char **restrict endptr);
long double strtold(const char *restrict s, char **restrict endptr);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 指向以 null 结尾的字符串。
- `endptr`: 可为 NULL；若非 NULL 则 `*endptr` 指向首个未解析字符。

**[Post-condition]:**
- Case 1 正常转换: 跳过空白，解析可选符号、可选 INF/NAN、浮点数字（含小数点、指数），返回浮点值。
- Case 2 无有效转换: 返回 0.0，`*endptr = s`。
- Case 3 上溢: 返回 ±HUGE_VAL，errno = ERANGE。
- Case 4 下溢: 返回次正规值或 0，errno = ERANGE。
- strtof/strtold 分别为 float/long double 版本，仅返回类型不同。

### 不变量

**[Invariant]:** 纯函数，不修改 `s` 内容。
使用 musl 内部的 `__floatscan` 完成实际浮点解析（共享于 scanf 系列）。

### 意图

将字符串转换为 double/float/long double 浮点数。核心解析逻辑位于 `__floatscan` (src/internal/floatscan.c)，支持十进制和十六进制浮点数（C99/C11 语法）。

### 系统算法

```
Phase 1: 跳过空白 (isspace)
Phase 2: 检测 INF/NAN/INFINITY 特殊值
Phase 3: 委托 __floatscan 解析浮点数
__floatscan 内部: 解析十进制/十六进制符号、整数部分、小数部分、指数部分，
使用多精度整数算术 (k_shift / hexfloat) 计算最终浮点值，
检测溢出/下溢并设 errno。
```

## 依赖关系

### 依赖的函数
- `strtox()`: 内部 static 函数，构建 FILE 包装器并委托给 __floatscan
- `__floatscan()`: musl 内部浮点扫描引擎（src/internal/floatscan.c），处理十进制和十六进制浮点数解析
- `sh_fromstring()`: 从字符串初始化 FILE 结构（内部, shgetc.h）
- `shlim()`: 设置读取限制（内部）
- `shcnt()`: 获取已读取字符数（内部）
- `isspace()`: 跳过前导空白（在 __floatscan 内处理）

### 依赖的数据结构
- `FILE f`: 栈上 FILE 结构体（伪装成文件流供扫描器使用）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `"shgetc.h"`: 字符串→FILE 适配器
- `"floatscan.h"`: __floatscan 声明
- `"stdio_impl.h"`: FILE 内部结构定义

### 被依赖
- atof: 直接包装调用
- scanf 系列: 通过 __floatscan 共享浮点解析
- wcstod: 宽字符浮点转换的内部后端
