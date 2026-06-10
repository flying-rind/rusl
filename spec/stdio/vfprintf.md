# vfprintf.c 规约

> musl libc 格式化输出核心引擎。实现 `vfprintf` 函数及所有内部辅助函数、状态机、类型系统。

---

## 依赖图

```
vfprintf
  ├─> printf_core (static)
  │     ├─> pop_arg (static) — 从 va_list 提取参数
  │     ├─> out (static) — 向 FILE 输出字节序列
  │     │     └─> __fwritex (see stdio_impl.h) — 无锁写
  │     ├─> pad (static) — 输出填充字符（空格/零）
  │     │     └─> out (static)
  │     ├─> fmt_x (static) — 格式化十六进制
  │     ├─> fmt_o (static) — 格式化八进制
  │     ├─> fmt_u (static) — 格式化十进制无符号数
  │     ├─> fmt_fp (static) — 格式化浮点数
  │     ├─> getint (static) — 解析格式串中整数
  │     ├─> strerror (see <string.h>) — %m 错误信息
  │     ├─> strnlen (see <string.h>) — %s 字符串长度
  │     └─> wctomb (see <wchar.h>) — 宽字符转换
  ├─> __towrite (see stdio_impl.h) — 准备流写入
  ├─> FLOCK/FUNLOCK (see stdio_impl.h) — 流锁定
  ├─> ferror (see <stdio.h>) — 检查流错误
  └─> va_copy (C99 variadic macro) — 复制 va_list
```

---

## 类型定义

### union arg

```c
union arg {
    uintmax_t i;
    long double f;
    void *p;
};
```

[Visibility]: Internal — 仅在 vfprintf.c 内使用

用于存储从 `va_list` 提取的任意类型的参数值

### 状态机枚举 (Internal Enum)

```c
enum { BARE, LPRE, LLPRE, HPRE, HHPRE, BIGLPRE, ZTPRE, JPRE, STOP,
       PTR, INT, UINT, ULLONG, LONG, ULONG, SHORT, USHORT, CHAR, UCHAR,
       LLONG, SIZET, IMAX, UMAX, PDIFF, UIPTR, DBL, LDBL, NOARG, MAXSTATE };
```

[Visibility]: Internal — 仅在 vfprintf.c 内使用

定义格式说明符状态机中所有状态。`STOP` 之前的值为"前缀状态"（如 `LPRE` = l 前缀），`STOP` 之后的值为"终结类型"（如 `INT`、`DBL` 等）。

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

[Visibility]: Internal — 仅在 vfprintf.c 内使用

使用位操作表示格式标志。每个标志对应一个 ASCII 字符，标志位 = `1 << (字符码 - ' ')`。

### static const unsigned char states[][]

[Visibility]: Internal — 仅在 vfprintf.c 内使用

格式说明符状态机表。8 个状态（BARE、LPRE、LLPRE、HPRE、HHPRE、BIGLPRE、ZTPRE、JPRE），每行定义各输入字符对应的下一个状态。输入超出范围时产生 `0` 结果（非法格式符）。

---

## 函数规约

### 1. pop_arg (static)

```c
static void pop_arg(union arg *arg, int type, va_list *ap);
```

[Visibility]: Internal — 不对外导出

#### Intent

根据类型标识 `type` 从 `va_list` 中提取对应类型的参数值并存入 `union arg`。

#### 前置条件

- `ap` 指向有效的 `va_list`，对应位置有正确类型的参数
- `type` 为状态机终结类型（PTR/INT/UINT/LONG/ULONG/…/LDBL）

#### 后置条件

- `arg->i` / `arg->f` / `arg->p` 包含提取的参数值
- `va_list` 指针前进至下一个参数

---

### 2. out (static)

```c
static void out(FILE *f, const char *s, size_t l);
```

[Visibility]: Internal — 不对外导出

#### Intent

向 `FILE` 流输出 `l` 字节数据。若流处于错误状态则不执行输出。

#### 前置条件

- `f` 指向有效的 `FILE` 对象（NULL 表示"仅计数"模式）
- `s` 指向至少 `l` 字节的可读数据

#### 后置条件

- 若 `f != NULL` 且 `!ferror(f)`，数据通过 `__fwritex` 写入流
- 若 `ferror(f)` 或 `f == NULL`，无操作（静默丢弃）

---

### 3. pad (static)

```c
static void pad(FILE *f, char c, int w, int l, int fl);
```

[Visibility]: Internal — 不对外导出

#### Intent

在宽度填充模式下输出填充字符 `c`，使总宽度达到 `w`。若 `l >= w` 或标志不允许填充则直接返回。

#### 前置条件

- 若 `f != NULL`，`f` 指向有效的 `FILE` 对象
- `c` 为填充字符（`' '` 或 `'0'`）

#### 后置条件

- 输出 `max(0, w - l)` 个字符 `c`
- 使用 256 字节栈缓冲区批量输出

---

### 4. fmt_x (static)

```c
static char *fmt_x(uintmax_t x, char *s, int lower);
```

[Visibility]: Internal — 不对外导出

#### Intent

将无符号整数 `x` 转换为十六进制字符串（逆向写入，从高位到低位）。`lower` 非零时生成小写字母。

#### 前置条件

- `s` 指向足够大的缓冲区末尾（逆向写入）

#### 后置条件

- 返回指向生成字符串起始位置的指针
- 若 `x == 0`，不写入任何字符（返回原始 `s`）

---

### 5. fmt_o (static)

```c
static char *fmt_o(uintmax_t x, char *s);
```

[Visibility]: Internal — 不对外导出

#### Intent

将无符号整数 `x` 转换为八进制字符串（逆向写入）。

#### 前置条件

- `s` 指向足够大的缓冲区末尾

#### 后置条件

- 返回指向生成字符串起始位置的指针

---

### 6. fmt_u (static)

```c
static char *fmt_u(uintmax_t x, char *s);
```

[Visibility]: Internal — 不对外导出

#### Intent

将无符号整数 `x` 转换为十进制字符串（逆向写入）。对超出 `ULONG_MAX` 的高位部分逐位取模处理。

#### 前置条件

- `s` 指向足够大的缓冲区末尾

#### 后置条件

- 返回指向生成字符串起始位置的指针

---

### 7. fmt_fp (static)

```c
static int fmt_fp(FILE *f, long double y, int w, int p, int fl, int t, int ps);
```

[Visibility]: Internal — 不对外导出

#### Intent

格式化浮点数 `y` 到 `FILE` 流，支持 `%e/%E/%f/%F/%g/%G/%a/%A` 所有浮点转换说明符。使用高精度大整数运算（基 10^9 的 `uint32_t` 数组）确保任意精度舍入正确。

#### 前置条件

- `y` 为待格式化的 `long double` 值
- `w` 为最小字段宽度，`p` 为精度（`<0` 表示默认精度 `6`）
- `fl` 为标志位集合，`t` 为转换类型字符，`ps` 为长度修饰符状态
- `LDBL_MANT_DIG == 53` 时 `sizeof(long double) == 8`（编译时断言）

#### 后置条件

- 返回值 = `MAX(w, 实际输出列数)`
- 若中间计算溢出（整数值范围），返回 `-1`
- 特殊值处理：
  - `NaN` → 输出 `"nan"` 或 `"NAN"`（取决于大小写标志）
  - `infinity` → 输出 `"inf"` 或 `"INF"`（取决于大小写标志）
- `%g/%G`：根据数值大小自动选择 `%e` 或 `%f` 格式

#### 系统算法

```
fmt_fp(f, y, w, p, fl, t, ps):
  1. 确定精度参数 (max_mant_dig / max_exp 取决于 LDBL vs DBL)
  2. 分配大整数缓冲区 big[bufsize]，元素为 uint32_t (基 10^9)
  3. 处理符号和标记前缀 "-" / "+" / " "
  4. 若 !isfinite(y)：输出 nan/inf，填充对齐，返回
  5. 提取指数 e2 = ilogb(y)，将有效数字乘以 2 归一化
  6. 若 %a/%A：生成十六进制浮点表示 "0xh.hhhhp±d"
  7. 否则：
     a. 将有效数字转换为基 10^9 的大整数
     b. 根据指数调整小数点位置（乘/除 2 的幂次）
     c. 舍入到指定精度
     d. 对 %g/%G 去除尾部零并选择格式
     e. 输出整数部分、小数点、小数部分、指数
  8. 填充对齐，返回宽度
```

---

### 8. getint (static)

```c
static int getint(char **s);
```

[Visibility]: Internal — 不对外导出

#### Intent

从格式字符串中解析十进制整数（用于字段宽度和精度）。指针 `*s` 前移超过已解析的数字。

#### 前置条件

- `*s` 指向以数字开头的字符串

#### 后置条件

- `*s` 指向第一个非数字字符
- 返回值 `i` 为解析的整数值，溢出时 `i = -1`

---

### 9. printf_core (static)

```c
static int printf_core(FILE *f, const char *fmt, va_list *ap,
                        union arg *nl_arg, int *nl_type);
```

[Visibility]: Internal — 不对外导出

#### Intent

printf 格式化引擎核心。支持两阶段处理：
- **Phase 1**（`f == NULL`）：解析格式字符串，提取 `$` 位置参数类型信息
- **Phase 2**（`f != NULL`）：执行实际格式化输出

#### 前置条件

- `fmt` 指向以 `'\0'` 结尾的有效格式化字符串
- `ap` 指向有效的 `va_list`
- Phase 1 时 `nl_arg` / `nl_type` 为输出数组（长度 `NL_ARGMAX+1`）
- Phase 2 时 `nl_arg` / `nl_type` 为已填充的参数数组

#### 后置条件

- Phase 1 (`f == NULL`)：
  - Case 字符/宽字符编码错误 → 返回 `-1`，`errno = EINVAL`
  - Case 无位置参数 → 返回 `0`
  - Case 有位置参数 → 返回 `1`，`nl_arg` / `nl_type` 已填充
- Phase 2 (`f != NULL`)：
  - Case 成功 → 返回写入的字节总数
  - Case 溢出 → 返回 `-1`，`errno = EOVERFLOW`
  - Case 非法格式符 → 返回 `-1`，`errno = EINVAL`
  - Case 编码错误 → 返回 `-1`

#### 系统算法

```
printf_core(f, fmt, ap, nl_arg, nl_type):
  cnt = 0
  while *fmt:
    l = 当前文本段长度
    if f: out(f, literal_text, l)
    cnt += l
    if *fmt == '\0': break

    // 解析 % 格式说明符
    处理位置参数 ($)
    解析标志位 (flags)
    解析字段宽度 (width)
    解析精度 (precision)
    状态机解析类型

    if f == NULL:
      // Phase 1: 仅提取类型信息
      记录 nl_arg / nl_type
      continue

    // Phase 2: 格式化输出
    switch 转换类型:
      %n: 写入当前计数到参数指针
      %p: 指针地址
      %x/%X/%o: 无符号十六进制/八进制
      %d/%i/%u: 有符号/无符号十进制
      %c: 字符
      %s: 字符串 (含 %m = strerror(errno))
      %C/%S: 宽字符/宽字符串
      %e/%f/%g/%a: 浮点数
    执行填充和输出

  return cnt
```

---

### 10. vfprintf

```c
int vfprintf(FILE *restrict f, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

向 `FILE` 流 `f` 写入格式化输出。是 `fprintf` 的 `va_list` 版本，printf 家族的核心入口。

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回写入的字符总数（不含 `'\0'`）
- Case 2 格式错误：返回 `-1`，`errno` 设置为 `EINVAL`
- Case 3 输出溢出：返回 `-1`，`errno` 设置为 `EOVERFLOW`
- Case 4 写入错误：返回 `-1`
- `f` 的写缓冲区已刷新，文件位置已更新
- `f` 的 `F_ERR` 标志保留原始状态

#### 系统算法

```
vfprintf(f, fmt, ap):
  1. va_copy(ap2, ap) 复制可变参数列表
  2. Phase 1: printf_core(NULL, fmt, &ap2, nl_arg, nl_type)
     解析格式串，提取位置参数信息
     若返回 < 0: 出错退出
  3. FLOCK(f) 获取流锁
  4. 清除并保存 f->flags 中的 F_ERR 位
  5. 若 f 无缓冲区(buf_size == 0):
     a. 保存原始缓冲区指针
     b. 设置临时内部缓冲区 internal_buf[80]
     c. 重置写指针
  6. 调用 __towrite(f) 初始化写模式
  7. Phase 2: printf_core(f, fmt, &ap2, nl_arg, nl_type)
     执行格式化输出
  8. 若使用了临时缓冲区:
     a. 调用 f->write(f, 0, 0) 冲刷输出
     b. 恢复原始缓冲区
  9. 检查 ferror(f) — 若有错误, ret = -1
  10. 恢复 F_ERR 标志
  11. FUNLOCK(f) 释放锁
  12. va_end(ap2)
  return ret
```

#### 不变量

- `f->flags` 中的 `F_ERR` 位在函数调用前后保持不变（由 Phase 2 可能产生的错误不影响流后续使用）
- 对无缓冲区流，始终使用 80 字节的 `internal_buf` 临时缓冲
- `va_copy` 确保原始 `va_list` 不被消耗

#### 依赖

- `printf_core()` (static) — 核心格式化引擎
- `__fwritex()` — 无锁写操作（见 `src/stdio/__fwritex.c`）
- `__towrite()` — 准备流写入模式（见 `src/stdio/__towrite.c`）
- `FLOCK` / `FUNLOCK` — 流锁定宏（见 `src/internal/stdio_impl.h`）
- `ferror()` — 检查流错误状态
- `strerror()` — `%m` 错误信息（见 `src/string/strerror.c`）
- `strnlen()` — 安全字符串长度（见 `src/string/strnlen.c`）
- `wctomb()` — 宽字符到多字节转换（见 `src/multibyte/wctomb.c`）
- `frexpl()` / `signbit()` / `scalbn()` / `isfinite()` — 浮点操作（见 `math.h`）
