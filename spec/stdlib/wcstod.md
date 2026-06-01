# wcstod 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wchar.h>

double wcstod(const wchar_t *restrict s, wchar_t **restrict endptr);
float wcstof(const wchar_t *restrict s, wchar_t **restrict endptr);
long double wcstold(const wchar_t *restrict s, wchar_t **restrict endptr);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
- `s`: 指向以 null 结尾的宽字符串。
- `endptr`: 可为 NULL。

**[Post-condition]:**
同 strtod，但操作对象为宽字符串（`wchar_t`）。跳过前导空白、解析浮点数、返回 double/float/long double。溢出时返回 ±HUGE_VAL 并设 errno=ERANGE。

### 不变量

**[Invariant]:** 纯函数。内部将宽字符串转换为多字节字符串后委托给 strtod。

### 意图

将宽字符串转换为浮点数。是 strtod 的宽字符版本。

### 系统算法

```
Phase 1: 使用 wcsrtombs 将宽字符串转为多字节字符串
Phase 2: 调用 strtod 完成实际解析
Phase 3: 若 endptr 非 NULL，将多字节偏移反向映射回宽字符偏移
```

## 依赖关系

### 依赖的函数
- `wcstox()`: 内部 static 函数，通过自定义 do_read 将宽字符串适配为 FILE 流
- `do_read()`: 内部 static 函数，宽字符→字节转换（非 ASCII 字符映射为 '@'）
- `__floatscan()`: musl 内部浮点扫描引擎（src/internal/floatscan.c）
- `sh_fromstring()`: 初始化 FILE 结构
- `shlim()`: 设置读取限制
- `shcnt()`: 获取读取字符数

### 依赖的数据结构
- `FILE f`: 栈上 FILE 结构体（f.cookie 指向宽字符串）

### 依赖的外部资源
- `<wchar.h>`: wchar_t 类型、函数声明
- `<wctype.h>`: 宽字符分类
- `"shgetc.h"`, `"floatscan.h"`, `"stdio_impl.h"`: 内部头文件

### 被依赖
- 应用层直接调用
