# iswctype / wctype 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wctype.h>
#include <string.h>

#define WCTYPE_ALNUM  1
#define WCTYPE_ALPHA  2
#define WCTYPE_BLANK  3
#define WCTYPE_CNTRL  4
#define WCTYPE_DIGIT  5
#define WCTYPE_GRAPH  6
#define WCTYPE_LOWER  7
#define WCTYPE_PRINT  8
#define WCTYPE_PUNCT  9
#define WCTYPE_SPACE  10
#define WCTYPE_UPPER  11
#define WCTYPE_XDIGIT 12

int iswctype(wint_t wc, wctype_t type);
wctype_t wctype(const char *s);
int __iswctype_l(wint_t c, wctype_t t, locale_t l);
wctype_t __wctype_l(const char *s, locale_t l);
int iswctype_l(wint_t c, wctype_t t, locale_t l);  // weak_alias
wctype_t wctype_l(const char *s, locale_t l);       // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
### iswctype
- `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
- `type`: 由 `wctype()` 返回的有效分类标识符。

### wctype
- `s`: 指向以 null 结尾的字符串，内容为分类名称。

**[Post-condition]:**
### iswctype
- Case 1: `type` 匹配某个已知分类且 `wc` 属于该分类 → 返回非零值。
- Case 2: `type` 不匹配任何已知分类或 `wc` 不属于该分类 → 返回 0。

### wctype
- Case 1: `s` 匹配已知分类名称（"alnum", "alpha", "blank", "cntrl", "digit", "graph", "lower", "print", "punct", "space", "upper", "xdigit"）→ 返回该分类的标识符（1-12）。
- Case 2: `s` 不匹配任何已知分类 → 返回 0。

### 不变量

**[Invariant]:** 纯函数。`names` 字符串常量不可变。分类标识符与 `names` 中的顺序严格对应。

### 意图

`iswctype` 是宽字符分类的通用化接口，将分类类型参数化。`wctype` 将分类名称字符串解析为分类标识符。二者共同实现 `<wctype.h>` 的可扩展字符分类机制。

### 系统算法

```
iswctype: 通过 switch 分发到对应的 isw* 函数，O(1)。
wctype: 遍历固定的 "names" 字符串（12个条目，每条目6字节），使用 strcmp 匹配，O(n)。
```
