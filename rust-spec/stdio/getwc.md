# getwc 函数规约

## 复杂度分级: Level 1

> musl libc 宽字符输入函数。等价于 `fgetwc(f)`，直接从 FILE 流读取一个宽字符。

---

## 函数接口

```rust
use core::ffi::c_uint;
use crate::internal::FILE;

extern "C" fn getwc(f: *mut FILE) -> c_uint; // wint_t
```

[Visibility]: `getwc` 是 `<wchar.h>` / `<stdio.h>` 标准库函数，对外导出。通常由宏实现，但 musl 同时提供函数实现以支持函数指针调用等场景。

Rust 侧实现策略：
- 直接委托给 `fgetwc(f)`，实现极为简单
- 作为转发代理，无独立逻辑

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 FILE 指针，指向已打开的流
- 流的方向必须为宽字符模式（若尚未设置，`fgetwc` 内部会调用 `fwide(f, 1)` 设置）

**[Post-condition]:**
- **Case 1 成功读取宽字符**
  - 返回读取到的宽字符（`wchar_t` 类型的 `wint_t` 值）
  - FILE 流位置前进

- **Case 2 到达文件末尾**
  - 返回 `WEOF`
  - FILE 流设置 `F_EOF` 标志

- **Case 3 读取错误或编码错误**
  - 返回 `WEOF`
  - FILE 流设置 `F_ERR` 标志
  - 编码错误时设置 `errno = EILSEQ`

**[Error Behavior]:**
- 同 `fgetwc`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从 FILE 流 `f` 中读取一个宽字符。等价于 `fgetwc(f)`。与 `fgetwc` 唯一区别在于某些实现中 `getwc` 可作为宏内联展开获得性能优化，但在 musl 中两者实现完全相同。

---

### 系统算法

```
getwc(f):
  return fgetwc(f)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
getwc (Public)
  └── fgetwc(f)  (see fgetwc.c spec)
```

---

## [RELY]

- `fgetwc` — 宽字符读取函数 (见 `fgetwc.md`)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getwc(f: *mut FILE) -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为等价于 `fgetwc`。
