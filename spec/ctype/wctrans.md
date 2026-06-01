# wctrans / towctrans 函数规约

## 复杂度分级: Level 2

---

## 函数接口

```c
#include <wctype.h>
#include <string.h>

wctrans_t wctrans(const char *class);
wint_t towctrans(wint_t wc, wctrans_t trans);
wctrans_t __wctrans_l(const char *s, locale_t l);
wint_t __towctrans_l(wint_t c, wctrans_t t, locale_t l);
wctrans_t wctrans_l(const char *s, locale_t l);       // weak_alias
wint_t towctrans_l(wint_t c, wctrans_t t, locale_t l); // weak_alias
```

### 前置/后置条件

**[Pre-condition]:**
### wctrans
- `class`: 指向以 null 结尾的字符串，内容为 "toupper" 或 "tolower"。

### towctrans
- `wc`: 类型为 `wint_t`，任意宽字符值。
- `trans`: 由 `wctrans()` 返回的变换描述符。

**[Post-condition]:**
### wctrans
- Case 1: `class` 为 "toupper" → 返回 `(wctrans_t)1`。
- Case 2: `class` 为 "tolower" → 返回 `(wctrans_t)2`。
- Case 3: 其他 → 返回 0。

### towctrans
- Case 1: `trans == 1`（"toupper"）→ 返回 `towupper(wc)`。
- Case 2: `trans == 2`（"tolower"）→ 返回 `towlower(wc)`。
- Case 3: 其他 → 返回 `wc`。

### 不变量

**[Invariant]:** 纯函数。变换描述符是固定整数（1 或 2），不与任何动态资源关联。线程安全。

### 意图

`wctrans` 将大小写变换名称字符串解析为变换描述符。`towctrans` 根据描述符执行对应的大小写变换。是 `<wctype.h>` 中宽字符大小写变换的通用接口。

### 系统算法

```
wctrans:
  strcmp(class, "toupper") == 0 → 返回 1
  strcmp(class, "tolower") == 0 → 返回 2
  否则 → 返回 0

towctrans:
  trans == 1 → towupper(wc)
  trans == 2 → towlower(wc)
  否则 → wc

时间复杂度: O(1)。
```
