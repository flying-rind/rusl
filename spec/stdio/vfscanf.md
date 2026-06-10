# vfscanf.c 规约

> musl libc 格式化输入核心引擎。实现 `vfscanf` 函数及所有内部辅助函数，支持 `%d`、`%s`、`%[`、`%f` 等全部 C 标准转换说明符。

---

## 依赖图

```
vfscanf
  ├─> store_int (static) — 将扫描值按长度修饰符存入目标
  ├─> arg_n (static) — 按位置参数索引提取 va_list 参数
  ├─> __intscan (see intscan.h) — 整数扫描核心
  ├─> __floatscan (see floatscan.h) — 浮点扫描核心
  ├─> __toread (see stdio_impl.h) — 准备流读取模式
  ├─> shlim / shgetc / shunget / shcnt (see shgetc.h) — 扫描辅助宏
  ├─> mbrtowc / mbsinit (see <wchar.h>) — 宽字符转换
  ├─> malloc / realloc / free (see <stdlib.h>) — %m 动态分配
  ├─> isspace / isdigit (see <ctype.h>) — 字符分类
  └─> FLOCK / FUNLOCK (see stdio_impl.h) — 流锁定
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

[Visibility]: Internal — 仅在 vfscanf.c 内使用

长度修饰符编码。负值表示比 `int` 更窄的类型，正值表示更宽的类型。

---

## 函数规约

### 1. store_int (static)

```c
static void store_int(void *dest, int size, unsigned long long i);
```

[Visibility]: Internal — 不对外导出

#### Intent

将无符号整数 `i` 按长度修饰符 `size` 截断并写入目标地址 `dest`。

#### 前置条件

- `dest != NULL`
- `size` 为 `SIZE_hh` / `SIZE_h` / `SIZE_def` / `SIZE_l` / `SIZE_ll` 之一

#### 后置条件

- `*dest` 包含截断后的整数值，类型为 `signed char` / `short` / `int` / `long` / `long long`

---

### 2. arg_n (static)

```c
static void *arg_n(va_list ap, unsigned int n);
```

[Visibility]: Internal — 不对外导出

#### Intent

从 `va_list` 中按位置参数索引 `n`（1-based）提取第 n 个 `void*` 参数。用于 `%n$` 位置参数支持。

#### 前置条件

- `ap` 已由 `va_start` 正确初始化
- 可变参数列表至少有 `n` 个参数

#### 后置条件

- 返回第 n 个参数作为 `void*` 指针
- 原始 `va_list` 不受影响（通过 `va_copy` 保护）

---

### 3. vfscanf

```c
int vfscanf(FILE *restrict f, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

#### Intent

从 `FILE` 流 `f` 读取格式化输入。是 `fscanf` 的 `va_list` 版本，scanf 家族的核心入口。支持位置参数（`%n$`）、赋值抑制（`%*`）、动态分配（`%m`）等扩展。

#### 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 由 `va_start` 正确初始化
- 流 `f` 处于读取模式

#### 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数（不含 `%n` 和赋值抑制的 `%*` 项）
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数
- Case 4 匹配失败：返回匹配失败前的成功赋值项数
- Case 5 动态分配失败（`%m` 的 `malloc`/`realloc`）：返回匹配数（其值为之前匹配的数目）

#### 系统算法

```
vfscanf(f, fmt, ap):
  FLOCK(f)
  if f->rpos == NULL: __toread(f)         // 初始化读取模式
  if f->rpos == NULL: goto input_fail     // 无法读取

  matches = 0, pos = 0
  for p in fmt:
    // 跳过空白字符
    if isspace(*p):
      跳过格式串中连续空白
      shlim(f, 0); 跳过输入流中对应空白
      continue

    // 字面量匹配（非%或%%）
    if *p != '%' or p[1] == '%':
      shlim(f, 0)
      if *p == '%': p++; 跳过输入空白
      c = shgetc(f)
      if c != *p: shunget(f); goto match_fail

    // 解析 % 格式说明符
    p++
    解析赋值抑制 '*': dest = NULL
    解析位置参数 '$': dest = arg_n(ap, n)
    否则: dest = va_arg(ap, void*)

    解析字段宽度 width
    解析动态分配 'm': alloc = !!dest

    解析长度修饰符 (h/hh/l/ll/L/j/z/t)
    解析转换类型 (d/i/o/u/x/c/s/[/n/p/a/e/f/g)

    switch 转换类型:
      %c, %s, %[:
        构建扫描集 scanset[257]
        %c: 所有字符可匹配
        %s: 排除空白字符
        %[: 按 [...] 或 [^...] 构建自定义扫描集

        若 SIZE_l 且 %S/%C:
          wchar_t 宽字符扫描，使用 mbrtowc()
        若 alloc (启用 %m):
          malloc/realloc 动态扩展缓冲区
        否则:
          直接写入 dest

        读取匹配字符直至扫描集排除 / 宽度用尽 / EOF
        if alloc: *(char/wchar_t**)dest = 缓冲区指针
        若非 %c: 追加 '\0' 终止符

      %d, %i, %o, %u, %x, %X, %p:
        x = __intscan(f, base, 0, ULLONG_MAX)
        if t=='p' and dest: *(void**)dest = (void*)x
        else: store_int(dest, size, x)

      %a, %e, %f, %g:
        y = __floatscan(f, size, 0)
        if dest: 按 size 写入 float/double/long double

      %n:
        store_int(dest, size, pos)
        continue  // 不增加匹配计数

    pos += shcnt(f)
    if dest: matches++   // 仅当有目标时增加匹配计数

  FUNLOCK(f)
  return matches

match_fail / input_fail / fmt_fail / alloc_fail:
  清理动态分配的内存(free s, free wcs)
  若 matches == 0: matches = -1 (EOF)
  FUNLOCK(f)
  return matches
```

#### 不变量

- 流 `f` 在函数开始时获取锁，在返回时释放锁（通过 `goto` 出口统一处理）
- `pos` 跟踪从流中读取的字符总数（用于 `%n`）
- 动态分配的内存（`%m`）在匹配失败时被释放
- `%n` 不计入匹配数

#### 依赖

- `__intscan()` — 整数扫描引擎（见 `src/internal/intscan.h`）
- `__floatscan()` — 浮点扫描引擎（见 `src/internal/floatscan.h`）
- `__toread()` — 准备流读取模式（见 `src/stdio/__toread.c`）
- `shlim` / `shgetc` / `shunget` / `shcnt` — 扫描缓冲区辅助（见 `src/internal/shgetc.h`）
- `mbrtowc()` / `mbsinit()` — 多字节到宽字符转换（见 `src/multibyte/mbrtowc.c`）
- `malloc()` / `realloc()` / `free()` — 动态内存管理（见 `src/malloc/`）
- `isspace()` / `isdigit()` — 字符分类（见 `src/ctype/`）
- `FLOCK` / `FUNLOCK` — 流锁定宏（见 `src/internal/stdio_impl.h`）
- `EOF` — 定义于 `<stdio.h>`

---

### 4. __isoc99_vfscanf (weak_alias)

```c
weak_alias(vfscanf, __isoc99_vfscanf);
```

[Visibility]: Internal — 不对外导出（musl 内部兼容别名）

- **Intention**: 提供 C99 标准兼容的 `__isoc99_vfscanf` 弱别名。与 `vfscanf` 行为完全相同。
