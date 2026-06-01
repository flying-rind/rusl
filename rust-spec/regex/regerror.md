# regerror Rust 接口规约

## 概述

本模块实现 POSIX `regerror()` 函数，将 `regcomp()` 或 `regexec()` 返回的 `REG_*` 错误码转换为经过 locale 处理的人类可读错误消息字符串。Rust 实现中，内部错误消息表可使用 Rust 安全抽象（如 `&[&CStr]` 静态切片）替代 C 的紧凑 `\0` 分隔字符串表，但对外 `regerror` 签名必须保持 ABI 兼容。

---

## 依赖图

```
regerror (Public)
  ├── ERROR_MESSAGES (Internal) — 静态错误消息表
  ├── __lctrans_cur (External)   — locale 翻译 [外部模块: locale]
  └── snprintf (External)        — libc 格式化输出
```

---

## [RELY]

Predefined Structures/Functions:
  `regex_t` (type, `<regex.h>`)                          // 依赖1: POSIX 正则类型（本函数忽略 preg 参数内容，仅保持签名兼容）
  `snprintf` (fn, libc)                                   // 依赖2: 安全格式化输出
  `__lctrans_cur` (fn, locale 模块)                       // 依赖3: 当前 locale 下的消息翻译
  `c_char`, `c_int`, `c_size_t` (std::ffi / libc 类型)    // 依赖4: C ABI 兼容类型

---

## [GUARANTEE]

Exported Interface:

```rust
extern "C" fn regerror(errcode: c_int, preg: *const regex_t, errbuf: *mut c_char, errbuf_size: size_t) -> size_t
```

本模块保证对外提供的接口签名，ABI 兼容 POSIX `regerror()`。

---

## 内部实现：错误消息表

### 设计思路

C 实现使用紧凑的 `static const char messages[]` 将所有 14 条错误消息以 `\0` 分隔串接在单一字节数组中，目的是避免动态重定位。在 Rust 中，我们可以利用静态生命周期的切片更安全地表达同一语义：使用 `&[&CStr]` 或 `&[&str]` 静态切片，以错误码直接索引，读取更直观且类型安全。

```rust
// [Visibility]: Internal — rusl crate 内部实现
pub(crate) static ERROR_MESSAGES: &[&str] = &[
    "No error",                                    // 索引 0: REG_OK
    "No match",                                    // 索引 1: REG_NOMATCH
    "Invalid regexp",                              // 索引 2: REG_BADPAT
    "Unknown collating element",                   // 索引 3: REG_ECOLLATE
    "Unknown character class name",                // 索引 4: REG_ECTYPE
    "Trailing backslash",                          // 索引 5: REG_EESCAPE
    "Invalid back reference",                      // 索引 6: REG_ESUBREG
    "Missing ']'",                                 // 索引 7: REG_EBRACK
    "Missing ')'",                                 // 索引 8: REG_EPAREN
    "Missing '}'",                                 // 索引 9: REG_EBRACE
    "Invalid contents of {}",                      // 索引 10: REG_BADBR
    "Invalid character range",                     // 索引 11: REG_ERANGE
    "Out of memory",                               // 索引 12: REG_ESPACE
    "Repetition not preceded by valid expression", // 索引 13: REG_BADRPT
];

pub(crate) static UNKNOWN_ERROR: &str = "Unknown error"; // 兜底消息
```

**不变量 (Invariants)**:
- `ERROR_MESSAGES` 的索引顺序必须与 `<regex.h>` 中 `REG_*` 错误码数值严格一致
- 对越界错误码（`errcode < 0` 或 `errcode >= ERROR_MESSAGES.len()`），返回 `UNKNOWN_ERROR`

---

## regerror (对外导出)

```rust
#[no_mangle]
pub unsafe extern "C" fn regerror(
    errcode: c_int,
    preg: *const regex_t,
    errbuf: *mut c_char,
    errbuf_size: size_t,
) -> size_t
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<regex.h>` 声明。用户程序可直接调用。

### 意图 (Intent)

将 `regcomp()` 或 `regexec()` 返回的 `REG_*` 错误码转换为对应的、经过当前 locale 处理的可读错误消息字符串，并将结果写入用户提供的缓冲区。

### 前置条件

- `errcode` 为 `regcomp()` 或 `regexec()` 的返回值，通常为 `<regex.h>` 中定义的 `REG_*` 宏值（0 ~ 13），但允许传入任意 `c_int` 值。
- `preg` 可为任意值（musl 实现中 `preg` 参数被完全忽略，调用者可传 `null()`；但 POSIX 标准要求传递有效的 `regex_t` 指针以保证可移植性）。
- 若 `errbuf` 非 `null()` 且 `errbuf_size > 0`，则 `errbuf` 指向的缓冲区至少有 `errbuf_size` 字节可写。
- 若 `errbuf_size == 0`，`errbuf` 可为 `null()`。

### 后置条件

**Case 1: `errbuf` 非 `null()` 且 `errbuf_size > 0`**
- `errbuf[0..min(errbuf_size-1, ret-1)]` 中写入以 `\0` 结尾的错误消息字符串（经过 locale 处理）。
- 若 `ret <= errbuf_size`，完整消息被写入（含结尾 `\0`）。
- 若 `ret > errbuf_size`，消息被截断至 `errbuf_size-1` 字节，`errbuf[errbuf_size-1]` 为 `\0`。

**Case 2: `errbuf == null()` 或 `errbuf_size == 0`**
- 不发生写入操作。
- 返回值仍然为完整消息所需的总字符数（含结尾 `\0`），如同 `snprintf(null(), 0, ...)` 的语义。

**通用**:
- 返回值 = 完整写入消息所需的字符数（含结尾 `\0`）。调用者可通过比较返回值与 `errbuf_size` 判断是否发生截断。

### 系统算法 (System Algorithm)

消息定位采用直接数组索引（Rust 实现优势）而非 C 实现的线性扫描：

```
let msg: &str = if errcode >= 0 && (errcode as usize) < ERROR_MESSAGES.len() {
    ERROR_MESSAGES[errcode as usize]
} else {
    UNKNOWN_ERROR
};
let translated = __lctrans_cur(msg);  // locale 翻译
let ret = snprintf(errbuf, errbuf_size, "%s\0", translated);
ret as size_t
```

**边界情况**：
- `errcode == 0`：返回 "No error"（索引 0）
- `errcode < 0`（如 `REG_ENOSYS = -1`）：返回 "Unknown error"
- `errcode >= 14`：返回 "Unknown error"

### 不变量 (Invariants)

- `regerror` 不修改 `preg` 指向的内容（即使传入非 NULL）
- `regerror` 无全局状态依赖（除 locale 翻译所需的线程局部数据）
- 多次调用 `regerror` 对同一错误码产生相同结果（locale 不变的前提下）

### 复杂度

- 时间复杂度：O(1) 数组索引 + O(n) snprintf 格式化（n = 消息长度）
- 空间复杂度：O(1)
