# vsnprintf — Rust 接口归约

## 原始 C 接口
```c
int vsnprintf(char *restrict s, size_t n, const char *restrict fmt, va_list ap);
```

## 内部函数（C）
```c
static size_t sn_write(FILE *f, const unsigned char *s, size_t l);
```

## 内部结构体（C）
```c
struct cookie { char *s; size_t n; };
```

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 是平台相关类型，只能通过 FFI 传递
extern "C" fn vsnprintf(
    s: *mut core::ffi::c_char,
    n: usize,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

注意：Rust 的 `VaList` 类型（`core::ffi::VaList`）自 Rust 1.71 起稳定，允许在 `extern "C"` 函数间传递 `va_list`。但构造 `VaList` 仍需通过 C 侧的 `va_start`（Rust 不支持可变参数的 `extern "C"` 函数中构造 `va_list`）。

---

## Rust 安全接口设计

由于 C 的 `va_list` 在 Rust 中无法凭空构造，Rust 内部应设计 **不依赖 va_list 的格式化引擎**：

```rust
// Rust 原生的格式化接口（非 C ABI）
pub fn rust_vsnprintf(
    buf: &mut [u8],        // 输出缓冲区（含 '\0' 预留位）
    fmt: &str,              // 格式化字符串
    args: &[FormatArg],     // 预解析的参数列表
) -> Result<usize, FmtError>;
```

其中 `FormatArg` 是一个枚举，替代 `va_list`：

```rust
// 模块内部定义，不对外暴露
pub(crate) enum FormatArg {
    Int(i64),
    Uint(u64),
    Float(f64),
    Char(u8),
    Str(*const u8, usize),   // 指针 + 长度
    Ptr(*const core::ffi::c_void),
}
```

---

## 意图
将格式化字符串按 `printf` 语义写入固定大小缓冲区，保证不溢出且始终 null 终止。

## 前置条件（安全接口）
- `buf.len() >= 1`（至少能容纳 `'\0'`）
- `fmt` 是有效 UTF-8 或 ASCII 字符串
- `args` 中的参数数量及类型与 `fmt` 中的格式说明符一致

## 后置条件
- 返回值 `Ok(len)` 为完整格式化后的字符串长度（不含 `'\0'`）
- 若 `len >= buf.len()`，则仅写入 `buf.len()-1` 字节的截断内容
- `buf[min(len, buf.len()-1)] == b'\0'`
- 若格式错误，返回 `Err(FmtError)`

## 不变量
- `wpos`（写入位置）始终不超过 `buf.len() - 1`
- 截断后的 `buf` 始终以 `'\0'` 终止

## 算法

原 C 实现通过构造 `FILE` 对象将输出委托给 `vfprintf`。Rust 中设计两套路径：

### 路径 A：C ABI 兼容（extern "C"）
1. 接收 `va_list ap`
2. 调用平台相关的格式化引擎（如直接复用 C 实现的 `vfprintf` 或使用 `libc::vsnprintf`）
3. 返回 `c_int`

### 路径 B：纯 Rust 实现（内部使用）
1. 创建 `BufWriter` 结构体包装 `&mut [u8]`
2. 逐字符解析 `fmt`：
   - 普通字符直接写入
   - `%s` → 写入字符串（截断超过容量部分）
   - `%d` / `%x` / `%p` → 整数转字符串后写入
   - `%%` → 写入 `%`
3. 每步检查剩余容量，超出则截断
4. 追加 `'\0'`，返回理论总长度

纯 Rust 实现不再依赖 `FILE` 和 `vfprintf`，复杂度约为 ~150 行。

```rust
struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,       // 当前写入位置
    total: usize,     // 理论总写入字符数（含截断部分）
}

impl<'a> BufWriter<'a> {
    fn write_byte(&mut self, b: u8) { /* ... */ }
    fn write_str(&mut self, s: &[u8]) { /* ... */ }
    fn write_int(&mut self, n: i64, radix: u32) { /* ... */ }
    fn finalize(self) -> usize { /* 追加 '\0'，返回 total */ }
}
```

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::ffi::VaList              // 依赖1: Rust 内置 va_list 类型（C ABI 路径）
  core::fmt::Write               // 依赖2: Rust 标准格式化 trait（可选路径）
Predefined Macros/Crates:
  (none)                          // 纯算法实现，无外部 crate 依赖

[GUARANTEE]
Exported Interface:
  extern "C" fn vsnprintf(
      s: *mut core::ffi::c_char,
      n: usize,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vsnprintf 符号
Internal Interface:
  pub(crate) fn rust_vsnprintf(
      buf: &mut [u8],
      fmt: &str,
      args: &[FormatArg],
  ) -> Result<usize, FmtError>;
                                 // 纯 Rust 格式化接口（供 crate 内部使用）
  pub(crate) enum FormatArg { ... }
                                 // 格式化参数类型（模块内部）
  pub(crate) struct BufWriter<'a> { ... }
                                 // 缓冲写入器（模块内部）