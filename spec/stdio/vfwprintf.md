# vfwprintf.c 规约

> musl libc 宽字符格式化输出核心引擎。实现 `vfwprintf` 函数及所有内部辅助函数、状态机、类型系统。与 `vfprintf.c` 的结构高度对称，区别在于格式字符串和终端输出均为宽字符。

---

## 依赖图

```
vfwprintf (Public)
  ├─> wprintf_core (static) — 宽字符格式化核心引擎
  │     ├─> pop_arg (static) — 从 va_list 提取参数
  │     ├─> out (static) — 向 FILE 输出宽字符
  │     │     └─> fputwc (see fputwc.c)
  │     ├─> pad (static) — 输出填充（使用 fprintf 打印空格）
  │     │     └─> fprintf (see fprintf.c)
  │     ├─> getint (static) — 解析宽字符格式串中整数
  │     │     └─> iswdigit (来自 <wctype.h>)
  │     ├─> strerror (来自 <string.h>) — %m 错误信息
  │     ├─> mbtowc (来自 <wchar.h>) — 字符串转换
  │     ├─> wcsnlen (来自 <wchar.h>) — 宽字符串安全长度
  │     ├─> snprintf (来自 <stdio.h>) — 构建 charfmt
  │     ├─> fprintf (来自 <stdio.h>) — 委托窄字符格式化
  │     └─> btowc (来自 <wchar.h>) — 单字节到宽字符
  ├─> fwide (see fwide.c) — 设置流方向
  ├─> FLOCK / FUNLOCK (来自 stdio_impl.h)
  ├─> ferror (来自 <stdio.h>)
  └─> va_copy (<stdarg.h>)
```

---

## 类型定义与宏

### union arg

```c
union arg {
    uintmax_t i;
    long double f;
    void *p;
};
```

[Visibility]: Internal — 仅在 vfwprintf.c 内使用

用于存储从 `va_list` 提取的任意类型的参数值。

### 格式标志位 (Internal Macros)

```c
#define ALT_FORM   (1U<<'#'-' ')
#define ZERO_PAD   (1U<<'0'-' ')
#define LEFT_ADJ   (1U<<'-'-' ')
#define PAD_POS    (1U<<' '-' ')
#define MARK_POS   (1U<<'+'-' ')
#define GROUPED    (1U<<'\''-' ')
#define FLAGMASK (ALT_FORM|ZERO_PAD|LEFT_ADJ|PAD_POS|MARK_POS|GROUPED)
```

[Visibility]: Internal — 仅在 vfwprintf.c 内使用

使用位操作表示格式标志。每个标志对应一个 ASCII 字符，标志位 = `1 << (字符码 - ' ')`。

### 状态机枚举 (Internal Enum)

```c
enum { BARE, LPRE, LLPRE, HPRE, HHPRE, BIGLPRE, ZTPRE, JPRE, STOP,
       PTR, INT, UINT, ULLONG, LONG, ULONG, SHORT, USHORT, CHAR, UCHAR,
       LLONG, SIZET, IMAX, UMAX, PDIFF, UIPTR, DBL, LDBL, NOARG, MAXSTATE };
```

[Visibility]: Internal — 仅在 vfwprintf.c 内使用

格式说明符状态机状态定义。`S(x)` 宏映射字符到数组索引。

### static const unsigned char states[][]

[Visibility]: Internal — 仅在 vfwprintf.c 内使用

格式说明符状态机表。8 个前缀状态（BARE/0 到 JPRE/7），每行定义各输入宽字符对应的下一状态。

### 类型大小前缀表 sizeprefix

```c
static const char sizeprefix['y'-'a'] = { ... };
```

[Visibility]: Internal — 仅在 vfwprintf.c 内使用

将浮点/整数类型的终端状态映射到 `snprintf` charfmt 所需的长度修饰符（如 `'a'` → `'L'`、`'d'` → `'j'`），用于委托给 `fprintf` 处理。

---

## 函数规约

### 1. pop_arg (static)

```c
static void pop_arg(union arg *arg, int type, va_list *ap);
```

[Visibility]: Internal — 不对外导出

同 `vfprintf.c` 中的 `pop_arg`。根据类型标识从 `va_list` 提取参数并存入 `union arg`。支持 `PTR`、`INT`、`UINT`、`LONG`、`ULONG`、`ULLONG`、`SHORT`、`USHORT`、`CHAR`、`UCHAR`、`LLONG`、`SIZET`、`IMAX`、`UMAX`、`PDIFF`、`UIPTR`、`DBL`、`LDBL`。

#### 依赖

- `va_arg` — C 标准可变参数提取宏

---

### 2. out (static)

```c
static void out(FILE *f, const wchar_t *s, size_t l);
```

[Visibility]: Internal — 不对外导出

#### Intent

向 FILE 流输出 `l` 个宽字符。若流处于错误状态则不执行输出。每个宽字符通过 `fputwc` 逐个输出。

#### 前置条件

- `f` 指向有效的 `FILE` 对象（NULL 表示"仅计数"模式）
- `s` 指向至少 `l` 个宽字符的可读数组

#### 后置条件

- 若 `f != NULL` 且 `!ferror(f)`，每个宽字符通过 `fputwc` 写入流
- 若 `ferror(f)` 或 `f == NULL`，无操作

---

### 3. pad (static)

```c
static void pad(FILE *f, int n, int fl);
```

[Visibility]: Internal — 不对外导出

#### Intent

输出 `n` 个空格作为填充。若 `fl & LEFT_ADJ`（左对齐）、`n <= 0` 或流已出错则直接返回。使用 `fprintf(f, "%*s", n, "")` 批量输出。

---

### 4. getint (static)

```c
static int getint(wchar_t **s);
```

[Visibility]: Internal — 不对外导出

#### Intent

从宽字符串格式串中解析十进制整数。使用 `iswdigit` 判断宽字符数字。溢出时返回 `-1`。指针 `*s` 前移超过已解析的数字。

---

### 5. wprintf_core (static)

```c
static int wprintf_core(FILE *f, const wchar_t *fmt, va_list *ap,
                         union arg *nl_arg, int *nl_type);
```

[Visibility]: Internal — 不对外导出

#### Intent

宽字符 printf 格式化引擎核心。与 `vfprintf.c` 的 `printf_core` 对称，但操作宽字符。支持两阶段处理：
- **Phase 1**（`f == NULL`）：解析格式字符串，提取 `$` 位置参数类型信息
- **Phase 2**（`f != NULL`）：执行实际格式化输出

核心策略：对于数值类型（`%d`、`%f` 等），构建窄字符 `charfmt` 格式串，委托给 `fprintf`（窄字符引擎）处理实际数字格式化。对于宽字符特有类型（`%C`、`%S`），自行处理。

#### 前置条件

- `fmt` 指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 指向有效的 `va_list`
- Phase 1 时 `nl_arg` / `nl_type` 为输出数组

#### 后置条件

- Phase 1 (`f == NULL`)：
  - Case 无位置参数 → 返回 `0`
  - Case 有位置参数 → 返回 `1`
  - Case 格式错误 → 返回 `-1`
- Phase 2 (`f != NULL`)：
  - Case 成功 → 返回写入的宽字符总数
  - Case 溢出 → 返回 `-1`，`errno = EOVERFLOW`
  - Case 非法格式符 → 返回 `-1`，`errno = EINVAL`
  - Case 编码错误 → 返回 `-1`
  - Case 流错误（`ferror(f)`）→ 返回 `-1`

#### 系统算法

```
wprintf_core(f, fmt, ap, nl_arg, nl_type):
  cnt = 0  // 输出字符计数器
  while *fmt:
    // 处理字面量文本段
    l = 字面量长度
    if f: out(f, 字面量宽字符, l)
    cnt += l
    if *fmt == '\0': break
    if (cnt > INT_MAX) goto overflow

    // 解析 % 格式说明符
    解析位置参数 ($)
    解析标志位 (flags)
    解析字段宽度 (width), 支持 *$ 间接宽度
    解析精度 (precision), 支持 .*$ 间接精度

    // 状态机解析类型 -> st
    do { st = states[st][S(*s++)] } while (st-1 < STOP)
    if (!st) goto inval

    // 提取参数
    if (st == NOARG):
      if (argpos >= 0) goto inval  // %m 不能有位置参数
    else:
      if (argpos >= 0): arg = nl_arg[argpos]
      else if (f): pop_arg(&arg, st, ap)
      else: return 0

    if (!f) continue  // Phase 1 结束

    // 检查流错误状态
    if (ferror(f)) return -1

    // 检查大小写: %C/%S 视为宽字符版本
    t = s[-1]
    if (ps && (t & 15) == 3) t &= ~32  // %C -> %c, %S -> %s

    switch t:
      '%n': 写入当前 cnt 到参数指针
      '%c'/'%C': 输出单个字符 (C: 宽字符, c: btowc 转换)
        填充左右
      '%S': 宽字符串输出
        计算长度: wcsnlen(a, p)
        填充左右, out(f, a, p)
      '%m': arg.p = strerror(errno)  // fall through
      '%s': 窄字符串输出
        通过 mbtowc 逐个将窄字符转换为宽字符输出
        逐个字符 mbtowc 转换, out(f, &wc, 1)
        计算宽度, 填充左右
      default:
        // 数值类型委托给 fprintf
        snprintf(charfmt, ...) 构建窄字符格式串
        fprintf(f, charfmt, w, p, arg.i/arg.f)

    cnt += l

  if (f) return cnt
  // Phase 1 末尾: 处理所有位置参数
  if (!l10n) return 0
  for i=1..NL_ARGMAX:
    提取 nl_arg[i]
  return 1

inval:  errno = EINVAL;  return -1
overflow: errno = EOVERFLOW; return -1
```

**注意**: 对于 `%s`，宽字符格式化引擎需要将窄字符串逐字符通过 `mbtowc` 转换为宽字符后再输出。这与 `vfprintf.c` 中的对称处理不同（后者直接输出字节到 FILE）。

#### 依赖

- `pop_arg()` (static) — 参数提取
- `out()` (static) — 宽字符输出
- `pad()` (static) — 宽度填充
- `getint()` (static) — 整数解析
- `states[][]` (static) — 状态机表
- `sizeprefix[]` (static) — 类型长度前缀映射
- `strerror()` — `%m` 错误信息（`<string.h>`）
- `mbtowc()` — 窄字符到宽字符转换（`<wchar.h>`）
- `wcsnlen()` — 宽字符串安全长度（`<wchar.h>`）
- `btowc()` — 单字节到宽字符转换（`<wchar.h>`）
- `snprintf()` / `fprintf()` — 窄字符格式化委托（`<stdio.h>`）
- `iswdigit()` — 宽字符数字判断（`<wctype.h>`）
- `F_ERR` — 文件错误标志（来自 `stdio_impl.h`）
- `EINVAL` / `EOVERFLOW` — 错误码（`<errno.h>`）
- `INT_MAX` / `NL_ARGMAX` — 溢出检测和参数上限（`<limits.h>`）
- `MB_LEN_MAX` — 多字节字符最大长度（`<limits.h>`）

---

### 6. vfwprintf

```c
int vfwprintf(FILE *restrict f, const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

向 FILE 流 `f` 写入宽字符格式化输出。是 `fwprintf` 的 `va_list` 版本，宽字符 printf 家族的核心入口。实现结构完全对称于 `vfprintf`。

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回写入的宽字符总数（不含 `L'\0'`）
- Case 2 格式错误：返回 `-1`，`errno = EINVAL`
- Case 3 输出溢出：返回 `-1`，`errno = EOVERFLOW`
- Case 4 写入错误：返回 `-1`
- `f->flags` 中的 `F_ERR` 标志在调用前后保持不变

#### 系统算法

```
vfwprintf(f, fmt, ap):
  1. va_copy(ap2, ap)
     // Phase 1: 仅解析格式串，提取位置参数信息
  2. if (wprintf_core(NULL, fmt, &ap2, nl_arg, nl_type) < 0):
       va_end(ap2); return -1
  3. FLOCK(f)
  4. fwide(f, 1)                          // 设置宽字符方向
  5. 保存并清除 f->flags 中的 F_ERR 位
  6. ret = wprintf_core(f, fmt, &ap2, nl_arg, nl_type)  // Phase 2: 实际输出
  7. if (ferror(f)) ret = -1
  8. 恢复 F_ERR 标志
  9. FUNLOCK(f)
  10. va_end(ap2)
  return ret
```

#### 不变量

- `f->flags` 中的 `F_ERR` 位在函数调用前后保持不变
- `va_copy` 确保原始 `va_list` 不被消耗
- 流方向被设置为宽字符模式

#### 依赖

- `wprintf_core()` (static) — 宽字符格式化核心引擎
- `fwide(FILE *, int)` — 流方向设置（见 `fwide.c`）
- `FLOCK` / `FUNLOCK` — 流锁定宏（来自 `stdio_impl.h`）
- `ferror()` — 检查流错误状态（`<stdio.h>`）
- `va_copy` / `va_end` — C99 可变参数宏（`<stdarg.h>`）
- `F_ERR` — 文件错误标志（来自 `stdio_impl.h`）
