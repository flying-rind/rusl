# wcstol 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wchar.h>
#include <inttypes.h>

long wcstol(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
long long wcstoll(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
unsigned long wcstoul(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
unsigned long long wcstoull(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
intmax_t wcstoimax(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
uintmax_t wcstoumax(const wchar_t *restrict s, wchar_t **restrict endptr, int base);
```

### 前置/后置条件

**[Visibility]:** Public

**[Pre-condition]:**
同 strtol 参数约定，但 `s` 为宽字符串，`endptr` 指向宽字符指针。

**[Post-condition]:**
同 strtol 族：成功返回转换值；无数字返回 0；溢出返回极值 + errno=ERANGE。

### 不变量

**[Invariant]:** 6 个函数通过宏模板共享核心逻辑。内部委托给 `__intscan`。

### 意图

将宽字符串转换为整数。strtol 族的宽字符版本，支持 6 种返回类型。

### 系统算法

```
宽字符串 → 逐一字符通过 iswdigit/iswspace/|32 映射 → __intscan 完成数制解析与溢出检测。
```

## 依赖关系

### 依赖的函数
- `do_read()`: 内部 static 函数，宽字符→字节转换
- `__intscan()`: musl 内部整数扫描引擎（src/internal/intscan.c）
- `sh_fromstring()`: 初始化 FILE 结构
- `shlim()`: 设置读取限制
- `shcnt()`: 获取读取字符数

### 依赖的数据结构
- `FILE f`: 栈上 FILE 结构体

### 依赖的外部资源
- `<wchar.h>`: wchar_t 类型
- `<inttypes.h>`: intmax_t/uintmax_t
- `<limits.h>`: 极值宏
- `<wctype.h>`: iswdigit/iswspace
- `"stdio_impl.h"`, `"intscan.h"`, `"shgetc.h"`: 内部头文件

### 被依赖
- 应用层直接调用
