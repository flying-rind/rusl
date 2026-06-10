# putwc 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符输出函数。等价于 `fputwc(c, f)`，直接向 FILE 流写入一个宽字符。

---

## 函数接口

```rust
use core::ffi::c_uint;
use crate::internal::FILE;

extern "C" fn putwc(c: c_uint /* wchar_t */, f: *mut FILE) -> c_uint; // wint_t
```

[Visibility]: `putwc` 是 `<wchar.h>` / `<stdio.h>` 标准库函数，对外导出。通常由宏实现，但 musl 同时提供函数实现以支持函数指针调用等场景。

Rust 侧实现策略：
- 直接委托给 `fputwc(c, f)`，实现极为简单
- 作为转发代理，无独立逻辑

---

### 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的宽字符（`wchar_t` 类型）
- `f`: 非空 FILE 指针，指向已打开的写模式流
- 流的方向必须为宽字符模式（若尚未设置，`fputwc` 内部会调用 `fwide(f, 1)` 设置）

**[Post-condition]:**
- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入流

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志

**[Error Behavior]:**
- 同 `fputwc`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

将宽字符 `c` 写入 FILE 流 `f`。等价于 `fputwc(c, f)`。与 `fputwc` 唯一区别在于某些实现中 `putwc` 可作为宏内联展开，但在 musl 中两者实现完全相同。

---

### 系统算法

```
putwc(c, f):
  return fputwc(c, f)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
putwc (Public)
  └── fputwc(c, f)  (see fputwc.c spec)
```

---

## [RELY]

- `fputwc` — 宽字符写入函数 (见 `fputwc.md`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn putwc(c: c_uint, f: *mut FILE) -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为等价于 `fputwc`。
