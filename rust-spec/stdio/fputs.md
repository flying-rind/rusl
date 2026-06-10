# fputs 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 字符串输出实现。将 C 字符串写入 FILE 流（不追加换行符）。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// fputs: 加锁版本
extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int;

// fputs_unlocked: 弱别名，与 fputs 共享同一实现
// weak_alias: fputs_unlocked 是 fputs 的弱别名
extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int;
```

[Visibility]:
- `fputs`: User — `<stdio.h>` 标准库函数，用户程序可直接调用。
- `fputs_unlocked`: User — POSIX 免锁 `fputs`，在 musl 中与 `fputs` 共享同一实现。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向以 `\0` 结尾的有效 C 字符串。
- `f`: 非空 `*mut FILE` 指针，指向已打开的写模式流。

**[Post-condition]:**
- **Case 1 成功写入完整字符串**
  - 返回非负值（具体为 0 或正值，musl 实现中若写入成功返回 0）。
  - 字符串内容已写入流。

- **Case 2 写入失败**
  - 返回 `EOF`（通常为 -1）。
  - `errno` 可能被设置。

**[Error Behavior]:**
- 写入失败时返回 `EOF`。不自动追加换行符（与 `puts` 的区别）。

---

### 不变量

**[Invariant]:**
- `fputs` 本身不添加换行符（与 `puts` 的区别）。
- 若成功，所有字符串字符（不含 `\0`）均已被写入或缓冲。

---

### 意图

将以 `\0` 结尾的 C 字符串 `s` 写入 FILE 流 `f`（不包括结尾的 `\0`，不自动追加换行符）。通过 `fwrite` 完成实际写入，返回非负值表示成功，`EOF` 表示失败。

Rust 侧实现：
- 先通过 `strlen`（或 Rust 中直接使用 `core::ffi::CStr::from_ptr` 获取长度）计算字符串长度。
- 委托给 `fwrite(s, 1, len, f)`，若 `fwrite` 返回的已写入字节数等于 `len` 则返回 0（成功），否则返回 `EOF`（-1）。
- 返回值技巧：`(fwrite(...) == l) as c_int - 1`，成功时 `(true as c_int) - 1 = 0`，失败时 `(false as c_int) - 1 = -1`。
- `fputs_unlocked` 在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

### 系统算法

```
fputs(s: *const c_char, f: *mut FILE) -> c_int:
  l = strlen(s)                             // 获取字符串长度(不含 \0)
  return if fwrite(s as *const c_void, 1, l, f) == l { 0 } else { EOF }
```

时间复杂度 O(n)，n 为字符串长度。

---

## 依赖图

```
fputs (Public)
  ├── strlen (from <string.h> 或 core::ffi::CStr)
  ├── fwrite (see fwrite spec)
  └── weak_alias → fputs_unlocked
```

---

## [RELY]

- `strlen` — 计算字符串长度（可使用 `core::ffi::CStr::from_ptr` 获取长度）。
- `fwrite(*const c_void, usize, usize, *mut FILE)` — 块写入函数。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn fputs(s: *const c_char, f: *mut FILE) -> c_int;`
  `extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 ISO C 标准 `fputs` 语义。`fputs_unlocked` 与 `fputs` 行为完全一致，为弱别名关系。
