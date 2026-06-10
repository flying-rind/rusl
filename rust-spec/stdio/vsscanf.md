# vsscanf — Rust 接口归约

## 复杂度分级: Level 2

> musl libc `va_list` 版字符串格式化输入函数。通过构造最小伪 `FILE` 对象并委托 `vfscanf` 实现。

---

## 原始 C 接口
```c
int vsscanf(const char *restrict s, const char *restrict fmt, va_list ap);
```

[Visibility]: User — 通过 `<stdio.h>` 对外导出

## 原始 C 内部函数（static）

| C 内部函数 | 用途 | Rust 重新设计方案 |
|-----------|------|-----------------|
| `string_read` (static) | 从字符串中读取的 FILE 回调 | 替换为 `StringReader` 结构体实现 `RustRead` trait |

---

## Rust 外部 ABI 接口

```rust
// C ABI 兼容: va_list 通过 core::ffi::VaList 传递
extern "C" fn vsscanf(
    s: *const core::ffi::c_char,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

## Rust 弱别名（C99 兼容）

```rust
// weak_alias: __isoc99_vsscanf 是 vsscanf 的弱别名
extern "C" fn __isoc99_vsscanf(
    s: *const core::ffi::c_char,
    fmt: *const core::ffi::c_char,
    ap: core::ffi::VaList,
) -> core::ffi::c_int;
```

[Visibility]: `vsscanf` 为 User 导出符号，`__isoc99_vsscanf` 为 Internal 符号（与 `vsscanf` 行为完全一致）。

---

## Rust 安全接口设计

```rust
// Rust 原生的 vsscanf 等价物——从内存字节切片读取
pub fn rust_vsscanf(input: &[u8], fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
```

内部实现通过构造 `StringReader` 适配器，其 `read` 方法从内存切片读取。然后将此适配器传入 `rust_scan_core` 引擎。

```rust
// 内部使用的内存字符串读取器（不对外暴露）
pub(crate) struct StringReader<'a> {
    data: &'a [u8],      // 源字节切片
    pos: usize,          // 当前读取位置
}

impl<'a> StringReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        // 查找 null 终止符位置，限制有效长度
        // data 中可能包含 '\0'，读取到 null 或数据末尾为止
        StringReader { data, pos: 0 }
    }
}

impl<'a> RustRead for StringReader<'a> {
    fn read_byte(&mut self) -> Option<u8> {
        if self.pos >= self.data.len() || self.data[self.pos] == b'\0' {
            None
        } else {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        }
    }

    fn unread_byte(&mut self, _byte: u8) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.read_byte() {
            if !b.is_ascii_whitespace() {
                self.unread_byte(b);
                break;
            }
        }
    }
}
```

---

## 意图

从内存中的 null 结尾字符串 `s` 读取格式化输入（`va_list` 版本）。是 `sscanf` 的 `va_list` 平替。

## 前置条件

- `s` 指向有效的 null 结尾 C 字符串
- `fmt != NULL`，指向有效的格式化字符串
- `ap` 已由 `va_start` 正确初始化

## 后置条件

- Case 1 成功：返回成功匹配并赋值的输入项数
- Case 2 输入失败（首个转换前到达字符串结尾）：返回 `EOF`
- `s` 源字符串不会被修改

## 不变量

- 伪 `FILE`/适配器对象仅在栈上存在，函数返回后销毁
- 源字符串 `s` 为只读，不被修改
- 无锁模式（`lock = -1`），伪流不会被多个线程共享

## 算法

原 C 实现：
```
vsscanf(s, fmt, ap):
  1. 在栈上构造 FILE 对象：
     .buf = (void *)s      // 缓冲区直接指向源字符串
     .cookie = (void *)s   // cookie 追踪读取位置
     .read = string_read   // 自定义读取回调
     .lock = -1            // 禁用锁定（伪流不会被共享）
  2. return vfscanf(&f, fmt, ap)
```

其中 `string_read` 通过 `memchr` 查找 `'\0'` 终止符确定可用数据长度。

Rust 实现路径：

### 路径 A：C ABI 兼容（extern "C"）
1. 在栈上构造 `FILE` 对象，设置 `buf`/`cookie`/`read`/`lock` 字段
2. 调用 `vfscanf(&f, fmt, ap)`（C ABI 路径）

### 路径 B：纯 Rust 实现（内部使用）
1. 构造 `StringReader { data: s_bytes }` 适配器
2. 解析 `FormatDest` 列表，调用 `scan_core(&mut reader, fmt, args)`
3. 返回成功匹配数

纯 Rust 实现不再依赖 `FILE` 结构体和自定义 `string_read` 回调，复杂度降低。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  int vfscanf(FILE *f, const char *fmt, va_list ap);
                                   // 依赖1: C ABI vfscanf 实现（核心引擎，C ABI 路径）
  struct FILE { ... }                // 依赖2: FILE 结构体定义（C ABI 路径）
  void *memchr(const void *s, int c, size_t n);
                                   // 依赖3: 查找 null 终止符（C 原 string_read 使用）
  core::ffi::VaList                  // 依赖4: Rust 内置 va_list 类型
  pub(crate) fn scan_core(input: &mut dyn RustRead, fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                   // 依赖5: Rust 扫描核心引擎（来自 vfscanf 模块）
  pub(crate) trait RustRead { ... }  // 依赖6: 读取抽象 trait（来自 vfscanf 模块）
  pub(crate) enum FormatDest { ... } // 依赖7: 格式化目标类型（来自 vfscanf 模块）
  pub(crate) enum ScanError { ... }  // 依赖8: 扫描错误类型（来自 vfscanf 模块）

[GUARANTEE]
Exported Interface:
  extern "C" fn vsscanf(
      s: *const core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // 本模块保证对外提供与 C ABI 兼容的 vsscanf 符号
  extern "C" fn __isoc99_vsscanf(
      s: *const core::ffi::c_char,
      fmt: *const core::ffi::c_char,
      ap: core::ffi::VaList,
  ) -> core::ffi::c_int;
                                 // C99 兼容弱别名（与 vsscanf 行为完全一致）
Internal Interface:
  pub fn rust_vsscanf(input: &[u8], fmt: &str, args: &mut [FormatDest]) -> Result<usize, ScanError>;
                                 // 安全的 Rust 原生格式化输入接口
  pub(crate) struct StringReader<'a> { ... }
                                 // 内存字符串读取适配器（模块内部）
