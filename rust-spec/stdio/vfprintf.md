# vfprintf — Rust 接口归约

## 复杂度分级: Level 3

> musl libc 格式化输出核心引擎。实现 `vfprintf` 函数及所有内部辅助函数。是所有 printf 家族函数的底层实现基础。

---

## 原始 C 接口
```c
int vfprintf(FILE *restrict f, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

## 原始 C 内部函数（static）

musl 的 `vfprintf.c` 包含以下内部静态函数，在 Rust 中可自由重新设计：

| C 内部函数 | 用途 | Rust 重新设计方案 |
|-----------|------|-----------------|
| `pop_arg` | 从 va_list 提取参数到 union arg | 替换为 `FormatArg` 数组直接索引 |
| `out` | 向 FILE 输出字节序列 | 替换为 `RustFile::write_all()` 安全方法 |
| `pad` | 输出填充字符 | 替换为 `RustFile::pad()` 方法 |
| `fmt_x` | 格式化十六进制 | 替换为 `format_hex()` 独立函数 |
| `fmt_o` | 格式化八进制 | 替换为 `format_oct()` 独立函数 |
| `fmt_u` | 格式化十进制无符号 | 替换为 `format_dec()` 独立函数 |
| `fmt_fp` | 格式化浮点数 | 替换为 Rust 浮点格式化（复用 `libm` 或纯整数实现） |
| `getint` | 解析格式串中整数 | 替换为 `parse_int()` / `str::parse()` |
| `printf_core` | 核心状态机 | 替换为 `format_core()` 纯 Rust 实现 |

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vfprintf(
    f: *mut FILE,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

## Rust 弱别名（C99 兼容）

```rust
// weak_alias: __isoc99_vfprintf 是 vfprintf 的弱别名
extern "C" fn __isoc99_vfprintf(
    f: *mut FILE,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

[Visibility]: `vfprintf` 为 User 导出符号，`__isoc99_vfprintf` 为 Internal 符号（与 `vfprintf` 行为完全一致，提供 C99 标准兼容别名）。

---

## Rust 安全接口设计

由于 C 的 `va_list` 在 Rust 中无法凭空构造，Rust 内部设计 **不依赖 va_list 的格式化引擎**：

```rust
// Rust 原生的格式化核心引擎（非 C ABI）
pub(crate) fn format_core(
    f: &mut dyn RustWrite,
    fmt: &str,
    args: &[FormatArg],
    nl_table: Option<&mut NlArgTable>,
) -> Result<usize, FmtError>;
```

参数类型：
```rust
// 模块内部定义
pub(crate) enum FormatArg {
    Int(i64),
    Uint(u64),
    Float(f64),
    LongDouble(f64),       // musl 使用 long double，在 Rust 中映射为 f64（如需 80-bit 精度则使用自定义类型）
    Char(u8),
    Str(*const u8, usize), // 指针 + 长度（%s）
    Ptr(*const core::ffi::c_void),  // 指针（%p）
    IntPtr(*mut core::ffi::c_int),  // int* 指针（%n 写入目标）
    None,                   // 无对应参数（格式错误时）
}

pub(crate) enum FmtError {
    InvalidFormat,          // 非法格式符 → errno = EINVAL
    Overflow,               // 输出计数溢出 → errno = EOVERFLOW
    WriteError,             // 底层写入失败
    EncodeError,            // 宽字符编码错误
}

pub(crate) trait RustWrite {
    fn write_all(&mut self, buf: &[u8]) -> Result<usize, WriteError>;
    fn write_byte(&mut self, b: u8) -> Result<(), WriteError>;
}

// 位置参数表（支持 %n$ 语法）
pub(crate) struct NlArgTable {
    types: [i32; NL_ARGMAX + 1],
    args: [FormatArg; NL_ARGMAX + 1],
}
```

---

## 意图

向 `FILE` 流 `f` 写入格式化输出。是 `fprintf` 的 `va_list` 版本，printf 家族的核心入口。

## 前置条件

- `f` 指向有效的 `FILE` 对象
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 由 `va_start` 正确初始化

## 后置条件

- Case 1 成功：返回写入的字符总数（不含 `'\0'`）
- Case 2 格式错误：返回 `-1`，`errno` 设置为 `EINVAL`
- Case 3 输出溢出：返回 `-1`，`errno` 设置为 `EOVERFLOW`
- Case 4 写入错误：返回 `-1`
- `f` 的写缓冲区已刷新，文件位置已更新
- `f` 的 `F_ERR` 标志保留原始状态

## 不变量

- `f->flags` 中的 `F_ERR` 位在函数调用前后保持不变
- 对无缓冲区流，始终使用 80 字节的临时缓冲区
- `va_copy` 确保原始 `va_list` 不被消耗（C ABI 路径）

## 算法

原 C 实现使用两阶段策略：

### Phase 1：类型推断
`printf_core(NULL, fmt, &ap2, nl_arg, nl_type)` — 解析格式字符串，提取位置参数类型信息，不产生输出。

### Phase 2：格式化输出
`printf_core(f, fmt, &ap2, nl_arg, nl_type)` — 执行实际格式化输出。

Rust 实现不再需要两阶段——因为 `FormatArg` 数组在调用前已完整，格式符和参数类型由调用者保证匹配。

### Rust 纯实现算法

```
fn format_core(f: &mut dyn RustWrite, fmt: &str, args: &[FormatArg], nl_table: Option<&mut NlArgTable>) -> Result<usize, FmtError>:
  total = 0
  arg_idx = 0
  
  while let Some(ch) = fmt.next() {
    if ch != '%' || fmt.next_is('%'):
      // 字面量字符
      f.write_byte(ch)
      total += 1
      continue
    
    // 解析 % 格式说明符
    parse_flags(&mut fmt)        // #, 0, -, ' ', +
    width = parse_width(&mut fmt)
    precision = parse_precision(&mut fmt)
    modifier = parse_modifier(&mut fmt)  // hh/h/l/ll/L/j/z/t
    
    match fmt.next() {
      'd' | 'i' => format_int(f, args[arg_idx], width, precision, flags)
      'u'        => format_uint(f, args[arg_idx], width, precision, flags, 10)
      'x' | 'X'  => format_uint(f, args[arg_idx], width, precision, flags, 16)
      'o'        => format_uint(f, args[arg_idx], width, precision, flags, 8)
      'p'        => format_ptr(f, args[arg_idx], width, flags)
      's'        => format_str(f, args[arg_idx], width, precision, flags)
      'c'        => format_char(f, args[arg_idx], width, flags)
      'f' | 'e' | 'g' | 'a' => format_float(f, args[arg_idx], width, precision, flags, ch)
      'n'        => store_count(args[arg_idx], total)  // 不增加 total
      '%'        => f.write_byte(b'%'); total += 1
      _          => return Err(FmtError::InvalidFormat)
    }
    
    arg_idx += 1  // 非 %n/%% 时
  }
  
  Ok(total)
```

内部整数格式化函数（替换 musl 的 `fmt_x`/`fmt_o`/`fmt_u`）：
```rust
// 纯 Rust 逆序写入，零分配
fn format_uint<W: RustWrite>(
    f: &mut W, value: u64, radix: u32, upper: bool,
    width: usize, precision: i32, flags: u32,
) -> Result<usize, FmtError> {
    let mut buf = [0u8; 65];  // 栈上缓冲区，最多 64-bit 二进制
    let s = buf_itoa(&mut buf, value, radix, upper);
    // 处理宽度填充、精度零填充、前缀 (0x/0/X)
    // ...
}
```

内部浮点格式化（替换 musl 的 `fmt_fp`）：
- 对于 `no_std` 环境，可依赖 `libm` crate 提供 `frexp`/`scalbn`/`isnan`/`isinf` 等基本浮点操作
- 高精度舍入使用大整数数组（基 10^9 的 `u32` 数组）确保任意精度
- 支持 `%e/%E/%f/%F/%g/%G/%a/%A` 全部浮点转换说明符

---

## 依赖图

原 C 依赖（`vfprintf` 核心引擎）：
```
vfprintf
  ├─> printf_core (static) — 在 Rust 中重新设计为 format_core()
  │     ├─> pop_arg (static) — 被 FormatArg 数组替代
  │     ├─> out (static) — 被 RustWrite trait 替代
  │     ├─> pad (static) — 被 format 函数内部逻辑替代
  │     ├─> fmt_x/fmt_o/fmt_u (static) — 被 format_uint() 替代
  │     ├─> fmt_fp (static) — 被 format_float() 替代
  │     └─> getint (static) — 被 parse_width()/parse_precision() 替代
  ├─> __fwritex — 被 RustWrite::write_all() 替代
  ├─> __towrite — 流写模式初始化
  ├─> FLOCK/FUNLOCK — 流锁定（RustFile 内部管理）
  ├─> ferror — 流错误检查
  ├─> strerror — %m 错误信息
  ├─> strnlen — %s 字符串长度
  └─> wctomb — 宽字符转换
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  struct FILE { ... }                // 依赖1: FILE 结构体定义（来自 stdio_impl.h）
  void __fwritex(FILE *f, const unsigned char *s, size_t l);
                                   // 依赖2: 无锁写操作（C ABI 路径）
  int __towrite(FILE *f);            // 依赖3: 准备流写入模式（C ABI 路径）
  int ferror(FILE *f);               // 依赖4: 检查流错误状态
  char *strerror(int errnum);        // 依赖5: %m 格式的错误信息
  size_t strnlen(const char *s, size_t n);
                                   // 依赖6: 安全字符串长度（%s）
  int wctomb(char *s, wchar_t wc);   // 依赖7: 宽字符到多字节转换（%lc）
  core::ffi::VaList                  // 依赖8: Rust 内置 va_list 类型（C ABI 路径）
  FLOCK / FUNLOCK 宏               // 依赖9: 流锁定（C ABI 路径）
  libm (frexp, scalbn, isnan, isinf) // 依赖10: 浮点基本操作（no_std 兼容）
  pub(crate) trait RustWrite { ... } // 依赖11: 写入抽象 trait（Rust 路径）
  pub(crate) enum FormatArg { ... }  // 依赖12: 格式化参数类型
  pub(crate) enum FmtError { ... }   // 依赖13: 格式化错误类型
  pub(crate) struct NlArgTable { ... }
                                   // 依赖14: 位置参数表

[GUARANTEE]
Exported Interface:
  extern "C" fn vfprintf(
      f: *mut FILE,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vfprintf 符号
  extern "C" fn __isoc99_vfprintf(
      f: *mut FILE,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // C99 兼容弱别名（与 vfprintf 行为完全一致）
Internal Interface:
  pub(crate) fn format_core(f: &mut dyn RustWrite, fmt: &str, args: &[FormatArg], nl: Option<&mut NlArgTable>) -> Result<usize, FmtError>;
                                 // 纯 Rust 格式化核心引擎
  pub(crate) fn format_uint<W: RustWrite>(f: &mut W, value: u64, radix: u32, ...) -> Result<usize, FmtError>;
                                 // 无符号整数格式化（十六进制/八进制/十进制）
  pub(crate) fn format_int<W: RustWrite>(f: &mut W, value: i64, ...) -> Result<usize, FmtError>;
                                 // 有符号整数格式化
  pub(crate) fn format_float<W: RustWrite>(f: &mut W, value: f64, ...) -> Result<usize, FmtError>;
                                 // 浮点数格式化
  pub(crate) fn format_str<W: RustWrite>(f: &mut W, s: &[u8], ...) -> Result<usize, FmtError>;
                                 // 字符串格式化
  pub(crate) fn format_ptr<W: RustWrite>(f: &mut W, ptr: *const c_void, ...) -> Result<usize, FmtError>;
                                 // 指针格式化
  pub(crate) fn format_char<W: RustWrite>(f: &mut W, ch: u8, ...) -> Result<usize, FmtError>;
                                 // 字符格式化
  pub(crate) enum FormatArg { ... }
                                 // 格式化参数类型
  pub(crate) trait RustWrite { ... }
                                 // 写入抽象 trait
  pub(crate) enum FmtError { ... }
                                 // 格式化错误类型
