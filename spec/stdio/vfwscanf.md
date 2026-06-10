# vfwscanf.c 规约

> musl libc 宽字符格式化输入核心引擎。实现 `vfwscanf` 函数及所有内部辅助函数。与 `vfscanf.c` 的结构高度对称，区别在于格式字符串和终端字符处理均为宽字符。

---

## 依赖图

```
vfwscanf (Public)
  ├─> store_int (static) — 按长度修饰符存储整数
  ├─> arg_n (static) — 按位置参数索引提取参数
  ├─> in_set (static) — 宽字符扫描集成员判断
  ├─> getwc / ungetwc (宏/函数, 来自 <wchar.h>)
  ├─> iswspace (来自 <wctype.h>)
  ├─> iswdigit (来自 <wctype.h>)
  ├─> wctomb (来自 <wchar.h>)
  ├─> snprintf (来自 <stdio.h>)
  ├─> fscanf (来自 <stdio.h>) — 委托窄字符数字扫描
  ├─> malloc / realloc / free (来自 <stdlib.h>) — %m 动态分配
  ├─> fwide (see fwide.c) — 设置流方向
  └─> FLOCK / FUNLOCK (来自 stdio_impl.h)

__isoc99_vfwscanf (weak_alias)
  └─> vfwscanf
```

---

## 内部宏定义

```c
#define SIZE_hh  -2
#define SIZE_h   -1
#define SIZE_def  0
#define SIZE_l    1
#define SIZE_L    2
#define SIZE_ll   3
```

[Visibility]: Internal — 仅在 vfwscanf.c 内使用

长度修饰符编码。负值表示比 `int` 更窄的类型。

### 宽字符 getwc/ungetwc 内联优化宏

```c
#undef getwc
#define getwc(f) \
    ((f)->rpos != (f)->rend && *(f)->rpos < 128 ? *(f)->rpos++ : (getwc)(f))

#undef ungetwc
#define ungetwc(c,f) \
    ((f)->rend && (c)<128U ? *--(f)->rpos : ungetwc((c),(f)))
```

[Visibility]: Internal — 仅在 vfwscanf.c 内使用

优化宏：当读缓冲区有数据且下一字节为 ASCII（`< 128`）时，直接通过缓冲区指针操作读写，避免函数调用开销。

---

## 函数规约

### 1. store_int (static)

```c
static void store_int(void *dest, int size, unsigned long long i);
```

[Visibility]: Internal — 不对外导出

同 `vfscanf.c` 中的 `store_int`。按长度修饰符将整数截断并写入目标地址。支持 `SIZE_hh` (`char`)、`SIZE_h` (`short`)、`SIZE_def` (`int`)、`SIZE_l` (`long`)、`SIZE_ll` (`long long`)。

---

### 2. arg_n (static)

```c
static void *arg_n(va_list ap, unsigned int n);
```

[Visibility]: Internal — 不对外导出

同 `vfscanf.c` 中的 `arg_n`。从 `va_list` 按位置参数索引 `n`（1-based）提取第 n 个参数。

---

### 3. in_set (static)

```c
static int in_set(const wchar_t *set, int c);
```

[Visibility]: Internal — 不对外导出

#### Intent

判断宽字符 `c` 是否属于扫描集 `set`。支持 `[a-z]` 范围表示法和 `[^...]` 反转（由调用者通过 `invert` 变量处理反转逻辑）。`set` 是遍历 `%[...]` 格式说明符中 `[` 和 `]` 之间的内容。

#### 系统算法

```
in_set(set, c):
  p = set
  if (*p == '-'):        // 首字符是 -
    if (c == '-'): return 1
    p++
  else if (*p == ']'):   // 首字符是 ]
    if (c == ']'): return 1
    p++
  for (; *p && *p != ']'; p++):
    if (*p == '-' && p[1] && p[1] != ']'):
      // 范围表示法 [a-z]
      for (j = p[-1]; j < *p; j++)
        if (c == j): return 1
      p++  // 跳过第二个字符 (for 循环的 p++ 完成)
    if (c == *p): return 1
  return 0
```

---

### 4. vfwscanf

```c
int vfwscanf(FILE *restrict f, const wchar_t *restrict fmt, va_list ap);
```

[Visibility]: User — `<stdarg.h>` / `<wchar.h>` 标准库函数，用户程序可直接调用

#### Intent

从 FILE 流 `f` 读取宽字符格式化输入。是 `fwscanf` 的 `va_list` 版本，宽字符 scanf 家族的核心入口。核心策略：
- 跳过空白字符：使用 `iswspace` 判断宽字符空白
- `%c` / `%s` / `%[`：直接操作宽字符进行匹配
- `%d` / `%f` 等数值类型：使用 `snprintf` 构建窄字符格式串，委托 `fscanf` 处理数字扫描
- `%S` / `%C`：视为 `%ls` / `%lc`（标准行为）
- `%m`：动态分配内存以接收输入

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向以 `L'\0'` 结尾的有效宽字符格式化字符串
- `ap` 由 `va_start` 正确初始化

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数（不含 `%n` 和赋值抑制项）
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数
- Case 4 匹配失败：返回匹配失败前的成功赋值项数
- Case 5 动态分配失败（`%m` 的 `malloc`/`realloc`）：返回当前已成功匹配的项数

#### 系统算法

```
vfwscanf(f, fmt, ap):
  FLOCK(f)
  fwide(f, 1)                        // 设置宽字符方向

  for p in fmt:
    alloc = 0

    // 跳过宽字符空白
    if iswspace(*p):
      while iswspace(p[1]): p++
      while iswspace(c = getwc(f)): pos++
      ungetwc(c, f)
      continue

    // 字面量匹配（非%或%%）
    if *p != '%' or p[1] == '%':
      if *p == '%': p++; while iswspace(c = getwc(f)): pos++
      else: c = getwc(f)
      if c != *p:
        ungetwc(c, f)
        if c < 0: goto input_fail
        goto match_fail
      pos++
      continue

    // 解析 % 格式说明符
    p++
    解析赋值抑制 '*': dest = NULL; p++
    解析位置参数 '$': dest = arg_n(ap, n); p+=2
    否则: dest = va_arg(ap, void*)

    解析字段宽度 width
    解析动态分配 'm': alloc = !!dest; p++

    // 长度修饰符
    size = SIZE_def
    switch *p++:
      'h':  size = (*p == 'h') ? (p++, SIZE_hh) : SIZE_h
      'l':  size = (*p == 'l') ? (p++, SIZE_ll) : SIZE_l
      'j':  size = SIZE_ll
      'z','t': size = SIZE_l
      'L':  size = SIZE_L

    // 区分大小写：%S/%C 转换为宽字符版本
    if ((t & 0x2f) == 3):  // t 为大写字母
      size = SIZE_l
      t |= 32              // 转小写

    // 跳过输入空白（%c 和 %[ 不跳过）
    if t != 'n' and t != '[' and (t|32) != 'c':
      while iswspace(c = getwc(f)): pos++
    else:
      c = getwc(f)
    if c < 0: goto input_fail
    ungetwc(c, f)

    switch t:
      '%n': store_int(dest, size, pos); continue

      '%s'/'%c'/'%[':
        // 确定扫描集
        if t == 'c': invert=1, set=L""
        else if t == 's': invert=1, set=spaces[]
        else: invert = (*++p == '^') ? (p++,1) : 0, set=p
              while (*p != ']'): if !*p goto fmt_fail; p++

        s = (size == SIZE_def) ? dest : NULL
        wcs = (size == SIZE_l) ? dest : NULL

        // 读取匹配字符
        i=0
        if (alloc):
          k = t=='c'?width+1:31
          if (size==SIZE_l): wcs = malloc(k*sizeof(wchar_t))
          else: s = malloc(k)
          if !wcs && !s: goto alloc_fail
        while (width):
          c = getwc(f)
          if c < 0: break
          if in_set(set, c) == invert: break  // 不匹配
          // 写入
          if wcs: wcs[i++] = c
          else if size != SIZE_l:
            l = wctomb(s ? s+i : tmp, c)
            if l < 0: goto input_fail
            i += l
          // 动态扩展
          if alloc && i >= k-4:
            k += k+1; realloc(...)
          pos++; width-=(width>0); gotmatch=1

        if width && t != 'c':
          ungetwc(c, f)
          if !gotmatch: goto match_fail

        if alloc:
          if size==SIZE_l: *(wchar_t**)dest = wcs
          else: *(char**)dest = s
        if t != 'c':
          if wcs: wcs[i]=0
          if s: s[i]=0

      '%d'/'%i'/'%o'/'%u'/'%x'/'%X':
      '%a'/'%e'/'%f'/'%g'/'%A'/'%E'/'%F'/'%G':
      '%p':
        // 委托给 fscanf 处理
        if width<1: width=0
        snprintf(tmp, ...) 构建窄格式串 "%.*s%*d%s%c%lln"
        cnt = 0
        if fscanf(f, tmp, dest?dest:&cnt, &cnt) == -1: goto input_fail
        else if !cnt: goto match_fail
        pos += cnt

      default: goto fmt_fail

    if dest: matches++

  FUNLOCK(f)
  return matches

fmt_fail / alloc_fail / input_fail:
  if (!matches) matches--
match_fail:
  if alloc: free(s), free(wcs)
  FUNLOCK(f)
  return matches
```

#### 不变量

- 流 `f` 在函数开始时获取锁，返回前释放锁
- `pos` 跟踪已读取的字符总数（用于 `%n`）
- `%m` 分配的内存失败时被释放
- 流方向被设置为宽字符模式
- 对整数/浮点/指针使用窄字符 `fscanf` 委托，仅对 `%s`、`%c`、`%[` 直接进行宽字符处理

#### 依赖

- `store_int()` (static) — 整数存储
- `arg_n()` (static) — 位置参数提取
- `in_set()` (static) — 扫描集判断
- `getwc` / `ungetwc` — 宽字符 I/O（`<wchar.h>`，本文件内联优化）
- `iswspace` / `iswdigit` — 宽字符分类（`<wctype.h>`）
- `wctomb()` — 宽字符到多字节转换（`<wchar.h>`）
- `snprintf()` / `fscanf()` — 窄字符委托（`<stdio.h>`）
- `malloc()` / `realloc()` / `free()` — 动态内存（`<stdlib.h>`）
- `fwide(FILE *, int)` — 流方向设置（见 `fwide.c`）
- `FLOCK` / `FUNLOCK` — 流锁定宏（来自 `stdio_impl.h`）
- `EOF` — 定义于 `<stdio.h>`

---

### 5. __isoc99_vfwscanf (weak_alias)

```c
weak_alias(vfwscanf, __isoc99_vfwscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部 C99 兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vfwscanf` 弱别名。与 `vfwscanf` 行为完全相同。
