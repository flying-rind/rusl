# vasprintf — Rust 接口归约

## 复杂度分级: Level 2

> musl libc `va_list` 版自动分配缓冲区的格式化输出函数。采用两阶段策略：先干跑计算长度，再 `malloc` 分配缓冲区完成写入。

---

## 原始 C 接口
```c
int vasprintf(char **s, const char *fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出（GNU 扩展 / POSIX）

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vasprintf(
    s: *mut *mut core::ffi::c_char,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

---

## Rust 安全接口设计

```rust
// Rust 原生的 vasprintf 等价物——返回堆分配的 String
pub fn rust_vasprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>;
```

内部实现采用与 C 相同的两阶段策略：

```rust
// Phase 1: 计算所需长度（干跑，不分配）
fn format_len(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;

// Phase 2: 分配缓冲区并写入
fn format_into(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
```

---

## 意图

将格式化字符串写入动态分配的缓冲区。缓冲区由 `malloc` 分配，调用者负责 `free`。采用两阶段策略：先干跑计算长度，再分配并写入。

## 前置条件

- `s != NULL`，`*s` 的值将被覆盖（不要求有效）
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

## 后置条件

- Case 1 成功：
  - `*s` 指向 `malloc` 分配的缓冲区，包含格式化后的 null 结尾字符串
  - 返回值为格式化字符串的长度（不含 `'\0'`）
  - 调用者有责任 `free(*s)`
- Case 2 长度计算失败（编码错误等）：返回 `-1`，`*s` 不变
- Case 3 `malloc` 失败：返回 `-1`，`*s` 不变

## 不变量

- 若 `vsnprintf(NULL, 0, ...)` 成功，其返回值精确等于"若缓冲区足够大时本应写入的字符数"
- `malloc(l+1)` 分配的缓冲区一定能容纳完整的格式化结果（包括 `'\0'`）

## 算法

原 C 实现：
```
vasprintf(s, fmt, ap):
  1. va_copy(ap2, ap) 复制 va_list
  2. l = vsnprintf(NULL, 0, fmt, ap2)  // Phase 1: 仅计算长度
     va_end(ap2)
  3. if l < 0: return -1                // 编码错误
  4. *s = malloc(l + 1)                 // 分配缓冲区 (+1 for '\0')
  5. if *s == NULL: return -1           // 分配失败
  6. return vsnprintf(*s, l + 1, fmt, ap)  // Phase 2: 写入
```

Rust 实现：

### 路径 A：C ABI 兼容（extern "C"）
遵循原 C 算法的两阶段策略，调用 `vsnprintf(NULL, 0, ...)` 和 `malloc`。

### 路径 B：纯 Rust 实现（内部使用）
```
fn rust_vasprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>:
  1. len = format_len(fmt, args)?  // Phase 1: 干跑计算长度
  2. let mut buf = Vec::with_capacity(len + 1)  // 分配缓冲区
  3. unsafe { buf.set_len(len + 1) }
  4. written = format_into(&mut buf[..len+1], fmt, args)?  // Phase 2: 写入
  5. Ok(RustString::from_vec(buf))
```

纯 Rust 实现不再依赖 `va_list` 复制，直接操作 `FormatArg` 数组。`format_len` 和 `format_into` 共享同一格式化逻辑，一个仅计数、一个实际写入。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vsnprintf(char *s, size_t n, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vsnprintf 实现（两阶段调用）
  void *malloc(size_t size);         // 依赖2: 动态内存分配
  core::ffi::VaList                  // 依赖3: Rust 内置 va_list 类型
  pub(crate) fn format_len(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖4: Phase 1 干跑计算（内部）
  pub(crate) fn format_into(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                   // 依赖5: Phase 2 缓冲写入（内部）
  pub(crate) enum FormatArg { ... }
                                   // 依赖6: 格式化参数类型（来自 vsnprintf 模块）
  pub(crate) struct RustString { ... }
                                   // 依赖7: no_std String 类型

[GUARANTEE]
Exported Interface:
  extern "C" fn vasprintf(
      s: *mut *mut core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vasprintf 符号
Internal Interface:
  pub fn rust_vasprintf(fmt: &str, args: &[FormatArg]) -> Result<RustString, FmtError>;
                                 // 安全的 Rust 原生格式化接口（堆分配）
  pub(crate) fn format_len(fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // Phase 1 长度计算（模块内部）
  pub(crate) fn format_into(buf: &mut [u8], fmt: &str, args: &[FormatArg]) -> Result<usize, FmtError>;
                                 // Phase 2 缓冲写入（模块内部）
