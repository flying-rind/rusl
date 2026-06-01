# iswalnum 函数规约

## 复杂度分级: Level 1

---

## 依赖图

```
iswalnum ──→ iswdigit (外部, see src/ctype/iswdigit.c)
         └─→ iswalpha (外部, see src/ctype/iswalpha.c)
__iswalnum_l ──→ iswalnum (本文件)
iswalnum_l ──→ weak_alias(__iswalnum_l)
```

---

## 函数接口

```c
#include <wctype.h>

int iswalnum(wint_t wc);
int __iswalnum_l(wint_t c, locale_t l);
int iswalnum_l(wint_t c, locale_t l);  // weak_alias(__iswalnum_l, iswalnum_l)
```

### 涉及的类型定义

```c
// 来自 <bits/alltypes.h> (通过 <wctype.h> 引入)
typedef unsigned __WINT_TYPE__ wint_t;

// 来自 alltypes.h.in (通过 <wctype.h> 引入)
// TYPEDEF struct __locale_struct * locale_t;
// locale_t 是指向 struct __locale_struct 的指针类型，具体定义见 src/internal/locale_impl.h
```

### 涉及的宏

```c
// 来自 <wctype.h>（非 C++ 编译时启用）
// #undef iswdigit
// #define iswdigit(a) (0 ? iswdigit(a) : ((unsigned)(a)-'0') < 10)
// 注意: 在 C 编译模式下 iswdigit 被定义为宏，因此 iswalnum 中对 iswdigit(wc)
// 的调用实际上会展开为内联的区间比较表达式，而非函数调用。
// 在 C++ 编译模式下保留为函数调用。

// 来自 <wctype.h>
#define WEOF 0xffffffffU
```

---

## iswalnum (对外导出)

**[Visibility]: Public** — POSIX.1-2001 标准函数，`<wctype.h>` 声明。

### 前置/后置条件

**[Pre-condition]:**
- `wc`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。

**[Post-condition]:**
- Case 1 — `wc` 是宽字符十进制数字或宽字符字母:
  - `iswdigit(wc)` 为真时，返回 1（数字分支优先，返回值固定为 1）。
  - `iswdigit(wc)` 为假但 `iswalpha(wc)` 为真时，返回非零值（具体值取决于 `iswalpha` 实现，musl 中为 1）。
- Case 2 — `wc` 既不是数字也不是字母，或 `wc == WEOF`:
  - 返回 0。

### 不变量

**[Invariant]:**
- 纯函数，无副作用，无内部状态，完全线程安全。
- 本实现不依赖 locale 设置（locale 参数 `l` 仅存在于 `_l` 后缀版本中，且被忽略）。

### 意图

判断宽字符是否属于字母或数字（POSIX "alnum" 字符类）。musl 实现采用"数字优先检测"策略：先执行代价极低的 `iswdigit` 检查（一次无符号范围比较），仅在该检查失败后才调用 `iswalpha` 进行位图查表——因为大多数输入文本中字母比数字更常见，但数字检查更快（无分支、无内存访问）。

### 系统算法

```
Step 1 — 数字快速路径:
  调用 iswdigit(wc):
    - C 编译模式下展开为宏: (unsigned)(wc) - L'0' < 10
    - 若成立 → 返回 1
    - 时间复杂度 O(1)，无分支（cmov 实现），无内存访问

Step 2 — 字母查表路径（仅在 Step 1 失败时执行）:
  调用 iswalpha(wc):
    - 见 src/ctype/iswalpha.c 规约（二级位图查表）
    - 返回 iswalpha 的结果（musl 中命中时为 1，未命中时为 0）

总体时间复杂度 O(1)，最坏情况仅 1 次无符号比较 + 1 次位图查表。
```

---

## __iswalnum_l (内部符号)

**[Visibility]: Internal (不导出)** — musl 内部辅助函数（`__` 前缀），POSIX/C 标准未定义。作为 `iswalnum_l` 的底层实现存在。

### 前置/后置条件

**[Pre-condition]:**
- `c`: 类型为 `wint_t`，任意宽字符值（含 `WEOF`）。
- `l`: 类型为 `locale_t`（`struct __locale_struct *`），指向有效的 locale 结构或为 `NULL`（表示 C locale）。

**[Post-condition]:**
- 完全等效于 `iswalnum(c)` 的返回值。
- musl 当前实现中 `locale_t l` 参数**被忽略**，所有 locale 下行为一致。

### 不变量

**[Invariant]:** 纯函数，无副作用。`l` 参数预留但未使用（musl 不支持 per-locale 字符分类）。

### 意图

POSIX locale-aware 字符分类函数的内部实现桩。musl 的宽字符分类基于 Unicode 且与 locale 无关，因此 `__iswalnum_l` 直接委托给 `iswalnum`，忽略 locale 参数。该设计确保与 POSIX 接口兼容的同时保持实现简洁。

---

## iswalnum_l (对外导出)

**[Visibility]: Public** — POSIX.1-2008 标准函数，`<wctype.h>` 声明。通过 `weak_alias(__iswalnum_l, iswalnum_l)` 机制暴露。

### 前置/后置条件

同 `__iswalnum_l`，完全一致的语义。

### 不变量

**[Invariant]:** `iswalnum_l` 是 `__iswalnum_l` 的弱别名，两者共享同一函数体。调用任一符号产生完全相同的行为。

---

## 跨文件依赖摘要

| 依赖符号 | 来源文件 | 可见性 | 说明 |
|---|---|---|---|
| `iswdigit` | `src/ctype/iswdigit.c` | Public | 见 `src/ctype/spec/iswdigit.md` |
| `iswalpha` | `src/ctype/iswalpha.c` | Public | 见 `src/ctype/spec/iswalpha.md` |
| `wint_t` | `<bits/alltypes.h>` (via `<wctype.h>`) | 系统类型 | `unsigned` 整型的 typedef |
| `locale_t` | `<bits/alltypes.h>` (via `<wctype.h>`) | 系统类型 | `struct __locale_struct *` 的 typedef |
| `WEOF` | `<wctype.h>` | 宏常量 | 值为 `0xffffffffU` |
