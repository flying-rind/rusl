# vfscanf — Rust 接口归约

## 复杂度分级: Level 3

> musl libc 格式化输入核心引擎。实现 `vfscanf` 函数及所有内部辅助函数。是所有 scanf 家族函数的底层实现基础。

---

## 原始 C 接口
```c
int vfscanf(FILE *restrict f, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

## 原始 C 内部函数和宏（static / Internal）

| C 内部定义 | 用途 | Rust 重新设计方案 |
|-----------|------|-----------------|
| `store_int` (static) | 将扫描值按长度修饰符截断并写入目标 | 替换为 `FormatDest::store_int(value, size)` 方法 |
| `arg_n` (static) | 按位置参数索引提取 va_list 参数 | 替换为 `FormatDest` 数组直接索引 |
| `SIZE_hh/SIZE_h/...` 宏 | 长度修饰符编码 | 替换为 Rust enum `SizeModifier` |

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vfscanf(
    f: *mut FILE,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

## Rust 弱别名（C99 兼容）

```rust
// weak_alias: __isoc99_vfscanf 是 vfscanf 的弱别名
extern "C" fn __isoc99_vfscanf(
    f: *mut FILE,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

[Visibility]: `vfscanf` 为 User 导出符号，`__isoc99_vfscanf` 为 Internal 符号（与 `vfscanf` 行为完全一致）。

---

## Rust 安全接口设计

由于 C 的 `va_list` 在 Rust 中无法凭空构造，Rust 内部设计 **不依赖 va_list 的扫描引擎**：

```rust
// Rust 原生的扫描核心引擎（非 C ABI）
pub(crate) fn scan_core(
    input: &mut dyn RustRead,
    fmt: &str,
    args: &mut [FormatDest],
) -> Result<usize, ScanError>;
```

参数类型：

```rust
// 模块内部定义
pub(crate) enum FormatDest {
    IntPtr(*mut core::ffi::c_int),          // %d / %i 目标
    ShortPtr(*mut core::ffi::c_short),      // %hd
    CharPtr(*mut core::ffi::c_char),        // %hhd
    LongPtr(*mut core::ffi::c_long),        // %ld
    LongLongPtr(*mut core::ffi::c_longlong), // %lld
    UintPtr(*mut core::ffi::c_uint),
    UlongPtr(*mut core::ffi::c_ulong),
    FloatPtr(*mut core::ffi::c_float),      // %f / %e / %g
    DoublePtr(*mut core::ffi::c_double),    // %lf
    StrPtr(*mut *mut core::ffi::c_char),    // %s 目标
    CharBuf(*mut core::ffi::c_char, usize), // %c 目标（缓冲区 + 大小）
    PtrPtr(*mut *mut core::ffi::c_void),    // %p 目标
    AllocStr(*mut *mut core::ffi::c_char),  // %ms 动态分配目标
    AllocWcs(*mut *mut wchar_t),             // %mls 宽字符动态分配
    None,                                     // 赋值抑制 %*（无目标）
}

pub(crate) enum SizeModifier {
    Hh,  // signed char / unsigned char
    H,   // short / unsigned short
    Default, // int / unsigned int
    L,   // long / unsigned long / double
    Ll,  // long long / unsigned long long
    Ld,  // long double (L modifier)
    J,   // intmax_t / uintmax_t
    Z,   // size_t / ssize_t
    T,   // ptrdiff_t
}

pub(crate) enum ScanError {
    InputFail,     // 首个转换前到达 EOF → 返回 EOF
    MatchFail,     // 部分匹配后失败 → 返回已匹配数
    FmtError,      // 格式字符串非法 → 返回已匹配数
    AllocFail,     // %m 的动态分配失败 → 返回已匹配数
}

pub(crate) trait RustRead {
    fn read_byte(&mut self) -> Option<u8>;
    fn unread_byte(&mut self, byte: u8);
    fn skip_whitespace(&mut self);
}
```

---

## 意图

从 `FILE` 流 `f` 读取格式化输入。是 `fscanf` 的 `va_list` 版本，scanf 家族的核心入口。支持位置参数（`%n$`）、赋值抑制（`%*`）、动态分配（`%m`）等扩展。

## 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 由 `va_start` 正确初始化
- 流 `f` 处于读取模式

## 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数（不含 `%n` 和赋值抑制的 `%*` 项）
- Case 2 输入失败（首个转换前到达 EOF）：返回 `EOF`
- Case 3 格式错误：返回已成功匹配的项数
- Case 4 匹配失败：返回匹配失败前的成功赋值项数
- Case 5 动态分配失败（`%m` 的 `malloc`/`realloc`）：返回匹配数（其值为之前匹配的数目）

## 不变量

- 流 `f` 在函数开始时获取锁，在返回时释放锁
- `pos` 跟踪从流中读取的字符总数（用于 `%n`）
- 动态分配的内存（`%m`）在匹配失败时被释放
- `%n` 不计入匹配数

## 算法

原 C 使用字符级状态机逐字符解析格式串。Rust 实现按格式说明符逐项处理。

### Rust 纯实现算法

```
fn scan_core(input: &mut dyn RustRead, fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>:
  matches = 0
  arg_idx = 0
  pos = 0  // 读取字符总数，用于 %n
  skip = false  // 赋值抑制标志（%*）
  alloc = false  // 动态分配标志（%m）
  
  for ch in fmt.chars():
    if ch.is_whitespace():
      skip_format_whitespace(&mut fmt)
      input.skip_whitespace()
      continue
    
    if ch != '%' || fmt.peek() == '%':
      // 字面量匹配
      input.skip_whitespace()
      c = input.read_byte()
      if c != Some(ch as u8): return Err(ScanError::MatchFail)
      pos += 1
      continue
    
    // 解析 % 格式说明符
    fmt.next()  // 跳过 '%'
    
    // 解析赋值抑制 '*'
    if fmt.peek() == '*': skip = true; fmt.next()
    
    // 解析位置参数 '$'
    // 解析字段宽度 width
    // 解析动态分配 'm': alloc = true
    
    // 解析长度修饰符
    size = parse_modifier(&mut fmt)
    
    // 解析转换类型
    match fmt.next():
      'c':
        width = if width == 0 { 1 } else { width }
        scan_chars(input, args[arg_idx], width, size)
      's':
        scan_string(input, args[arg_idx], width, alloc, size)
      '[':
        scanset = parse_scanset(&mut fmt)
        scan_scanset(input, args[arg_idx], scanset, width, alloc, size)
      'd' | 'i' | 'o' | 'u' | 'x' | 'X' | 'p':
        scan_int(input, args[arg_idx], base, size)
      'a' | 'e' | 'f' | 'g':
        scan_float(input, args[arg_idx], size)
      'n':
        store_count(args[arg_idx], pos, size)
        continue  // 不增加匹配计数
      _:
        return Err(ScanError::FmtError)
    
    if !skip && !is_count: matches += 1
    arg_idx += 1
  
  Ok(matches)
```

### 内部扫描函数

```rust
// 整数扫描（委托给 intscan 引擎）
fn scan_int<R: RustRead>(
    input: &mut R,
    dest: &mut FormatDest,
    base: u32,
    size: SizeModifier,
) -> Result<(), ScanError>;

// 浮点扫描（委托给 floatscan 引擎）
fn scan_float<R: RustRead>(
    input: &mut R,
    dest: &mut FormatDest,
    size: SizeModifier,
) -> Result<(), ScanError>;

// 字符串扫描（含动态分配 %ms 支持）
fn scan_string<R: RustRead>(
    input: &mut R,
    dest: &mut FormatDest,
    width: usize,
    alloc: bool,
    size: SizeModifier,
) -> Result<(), ScanError>;

// 字符扫描（%c）
fn scan_chars<R: RustRead>(
    input: &mut R,
    dest: &mut FormatDest,
    width: usize,
    size: SizeModifier,
) -> Result<(), ScanError>;

// 扫描集扫描（%[...] / %[^...]）
fn scan_scanset<R: RustRead>(
    input: &mut R,
    dest: &mut FormatDest,
    scanset: &Scanset,
    width: usize,
    alloc: bool,
    size: SizeModifier,
) -> Result<(), ScanError>;
```

### 长度修饰符 → 存储处理

```rust
impl FormatDest {
    fn store_int(&self, value: u64, size: SizeModifier) {
        match (self, size) {
            (FormatDest::CharPtr(p), SizeModifier::Hh) => unsafe { **p = value as i8 },
            (FormatDest::ShortPtr(p), SizeModifier::H) => unsafe { **p = value as i16 },
            (FormatDest::IntPtr(p), SizeModifier::Default) => unsafe { **p = value as i32 },
            (FormatDest::LongPtr(p), SizeModifier::L) => unsafe { **p = value as i64 },
            (FormatDest::LongLongPtr(p), SizeModifier::Ll) => unsafe { **p = value as i64 },
            // ...
        }
    }
    
    fn store_float(&self, value: f64, size: SizeModifier) {
        match (self, size) {
            (FormatDest::FloatPtr(p), _) => unsafe { **p = value as f32 },
            (FormatDest::DoublePtr(p), SizeModifier::L) => unsafe { **p = value },
            // long double 需要特殊处理
        }
    }
}
```

---

## 依赖图

原 C 依赖：
```
vfscanf
  ├─> store_int (static) — 被 FormatDest::store_int() 替代
  ├─> arg_n (static) — 被 FormatDest 数组直接索引替代
  ├─> __intscan — 整数扫描引擎（内部复用或纯 Rust 重写）
  ├─> __floatscan — 浮点扫描引擎（内部复用或纯 Rust 重写）
  ├─> __toread — 准备流读取模式
  ├─> shlim/shgetc/shunget/shcnt — 被 RustRead trait 替代
  ├─> mbrtowc/mbsinit — 宽字符转换
  ├─> malloc/realloc/free — %m 动态分配
  ├─> isspace/isdigit — 字符分类
  └─> FLOCK/FUNLOCK — 流锁定
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  struct FILE { ... }                // 依赖1: FILE 结构体定义（来自 stdio_impl.h）
  unsigned long long __intscan(FILE *f, unsigned base, int pok, unsigned long long lim);
                                   // 依赖2: 整数扫描引擎（来自 intscan.h）
  long double __floatscan(FILE *f, int prec, int pok);
                                   // 依赖3: 浮点扫描引擎（来自 floatscan.h）
  int __toread(FILE *f);             // 依赖4: 准备流读取模式
  size_t mbrtowc(wchar_t *wc, const char *s, size_t n, mbstate_t *ps);
                                   // 依赖5: 宽字符转换（%ls / %lc）
  int isspace(int c);                // 依赖6: 空白字符检测
  int isdigit(int c);                // 依赖7: 数字检测
  void *malloc(size_t size);         // 依赖8: 动态分配（%ms）
  void *realloc(void *ptr, size_t size);
                                   // 依赖9: 重新分配（%ms 扩展）
  void free(void *ptr);              // 依赖10: 释放（%ms 清理）
  core::ffi::VaList                  // 依赖11: Rust 内置 va_list 类型（C ABI 路径）
  FLOCK / FUNLOCK 宏               // 依赖12: 流锁定（C ABI 路径）
  pub(crate) trait RustRead { ... }  // 依赖13: 读取抽象 trait（Rust 路径）
  pub(crate) enum FormatDest { ... } // 依赖14: 格式化目标类型
  pub(crate) enum ScanError { ... }  // 依赖15: 扫描错误类型
  pub(crate) enum SizeModifier { ... }
                                   // 依赖16: 长度修饰符枚举
  pub(crate) struct Scanset { ... }  // 依赖17: %[...] 扫描集

[GUARANTEE]
Exported Interface:
  extern "C" fn vfscanf(
      f: *mut FILE,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vfscanf 符号
  extern "C" fn __isoc99_vfscanf(
      f: *mut FILE,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // C99 兼容弱别名（与 vfscanf 行为完全一致）
Internal Interface:
  pub(crate) fn scan_core(input: &mut dyn RustRead, fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                 // 纯 Rust 扫描核心引擎
  pub(crate) fn scan_int<R: RustRead>(input: &mut R, dest: &mut FormatDest, base: u32, size: SizeModifier) -> Result<(), ScanError>;
                                 // 整数扫描
  pub(crate) fn scan_float<R: RustRead>(input: &mut R, dest: &mut FormatDest, size: SizeModifier) -> Result<(), ScanError>;
                                 // 浮点扫描
  pub(crate) fn scan_string<R: RustRead>(input: &mut R, dest: &mut FormatDest, width: usize, alloc: bool, size: SizeModifier) -> Result<(), ScanError>;
                                 // 字符串扫描
  pub(crate) fn scan_scanset<R: RustRead>(input: &mut R, dest: &mut FormatDest, scanset: &Scanset, width: usize, alloc: bool, size: SizeModifier) -> Result<(), ScanError>;
                                 // 扫描集扫描
  pub(crate) enum FormatDest { ... }
                                 // 格式化目标类型
  pub(crate) trait RustRead { ... }
                                 // 读取抽象 trait
  pub(crate) enum ScanError { ... }
                                 // 扫描错误类型
  pub(crate) enum SizeModifier { ... }
                                 // 长度修饰符
  pub(crate) struct Scanset { ... }
                                 // 扫描集（%[...]）
