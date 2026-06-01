# strtol 函数规约

## 复杂度分级: Level 3

---

## 函数接口

```c
#include <stdlib.h>
#include <inttypes.h>

long strtol(const char *restrict s, char **restrict endptr, int base);
long long strtoll(const char *restrict s, char **restrict endptr, int base);
unsigned long strtoul(const char *restrict s, char **restrict endptr, int base);
unsigned long long strtoull(const char *restrict s, char **restrict endptr, int base);
intmax_t strtoimax(const char *restrict s, char **restrict endptr, int base);
uintmax_t strtoumax(const char *restrict s, char **restrict endptr, int base);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 待解析字符串。
- `endptr`: 可为 NULL。
- `base`: 进制（0 或 2-36），0 表示自动检测（0x 前缀为 16，0 前缀为 8，否则为 10）。

**[Post-condition]:**
- Case 1 成功: 返回转换后的值，`*endptr` 指向首个非数字字符。
- Case 2 无数字: 返回 0，`*endptr = s`（若无 endptr 则为语义等价）。
- Case 3 溢出 (有符号): 返回目标类型极值 (LONG_MIN/MAX 等)，errno = ERANGE。
- Case 3 溢出 (无符号): 返回 Uxxx_MAX，errno = ERANGE。

### 不变量

**[Invariant]:** 无全局状态。6 个函数通过宏 `strtox()` 共享同一份核心实现 `__strtox_internal`。
宏每次实例化生成一套专用于特定返回类型的内联代码。

### 意图

将字符串转换为整数。支持 6 种返回类型（long/long long/unsigned long/unsigned long long/intmax_t/uintmax_t），通过宏模板生成以减少代码冗余。核心字母转数值技巧: `(c | 32) - 'a' + 10`。

### 系统算法

```
Phase 1: 跳过空白 (isspace)
Phase 2: 检测符号
Phase 3: 自动检测 base（若无 0x/0 前缀则为 10）
Phase 4: 核心循环逐字符累加 n = n * base + digit
  - digit 转换: 0-9 直接减 '0'，a-z/A-Z 通过 |32 转小写后计算
  - 溢出检测: cutoff = 目标类型 MAX / base, cutlim = MAX % base
  - 若 n > cutoff 或 n == cutoff && digit > cutlim → 设溢出标志
Phase 5: 根据溢出标志和符号返回极值并设 errno=ERANGE
```

## 依赖关系

### 依赖的函数
- `strtox()`: 内部 static 模板函数，构建 FILE 包装器并委托给 __intscan
- `__intscan()`: musl 内部整数扫描引擎（src/internal/intscan.c），处理任意进制解析和溢出检测
- `sh_fromstring()`: 从字符串初始化 FILE 结构（内部, shgetc.h）
- `shlim()`: 设置读取限制（内部）
- `shcnt()`: 获取已读取字符数（内部）
- `isspace()`: 跳过前导空白（__intscan 内处理）

### 依赖的数据结构
- `FILE f`: 栈上 FILE 结构体（伪装成文件流）

### 依赖的外部资源
- `<stdlib.h>`: 函数声明
- `<inttypes.h>`: intmax_t/uintmax_t 类型
- `<limits.h>`: LONG_MIN/MAX, LLONG_MIN/MAX, ULONG_MAX, ULLONG_MAX
- `<ctype.h>`: isspace
- `"stdio_impl.h"`: FILE 内部结构
- `"intscan.h"`: __intscan 声明
- `"shgetc.h"`: 字符串→FILE 适配器

### 被依赖
- atoi/atol/atoll: 可视为简化版包装（独立实现）
- wcstol 族: 宽字符版本的后端
- scanf 系列: 通过 __intscan 共享整数解析
