# getwchar 函数规约

## 复杂度分级: Level 1

> musl libc 标准输入宽字符读取函数。从 `stdin` 读取一个宽字符。

---

## 函数接口

```rust
use core::ffi::c_uint;

// getwchar — User, 标准库函数
extern "C" fn getwchar() -> c_uint; // wint_t

// getwchar_unlocked — User (weak_alias -> getwchar)
extern "C" fn getwchar_unlocked() -> c_uint;
```

[Visibility]: `getwchar` 是 `<wchar.h>` 标准库函数，对外导出。`getwchar_unlocked` 是 POSIX 免锁扩展，通过弱别名对外导出（在 musl 中与 `getwchar` 指向同一实现）。

Rust 侧实现策略：
- 直接委托给 `fgetwc(stdin)`，实现极为简单
- `stdin` 通过全局访问器宏或函数获取

---

### 前置/后置条件

**[Pre-condition]:**
- `stdin` 已正确初始化并处于可读状态

**[Post-condition]:**
- **Case 1 成功读取宽字符**
  - 返回读取到的宽字符（`wchar_t` 类型的 `wint_t` 值）
  - `stdin` 流位置前进

- **Case 2 到达文件末尾**
  - 返回 `WEOF`
  - `stdin` 设置 `F_EOF` 标志

- **Case 3 读取错误或编码错误**
  - 返回 `WEOF`
  - `stdin` 设置 `F_ERR` 标志
  - 编码错误时设置 `errno = EILSEQ`

**[Error Behavior]:**
- 同 `fgetwc`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

从标准输入流 `stdin` 读取一个宽字符。等价于 `getwc(stdin)` 或 `fgetwc(stdin)`。

---

### 系统算法

```
getwchar():
  return fgetwc(stdin)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
getwchar (Public)
  └── fgetwc(stdin)  (see fgetwc.c spec)

getwchar_unlocked (weak_alias)
  └── getwchar

stdin (全局变量, 来自 <stdio.h>)
```

---

## [RELY]

- `fgetwc` — 宽字符读取函数 (见 `fgetwc.md`)
- `stdin` — 标准输入 FILE 指针 (`<stdio.h>` 全局变量)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getwchar() -> c_uint;`
  `extern "C" fn getwchar_unlocked() -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号。`getwchar_unlocked` 为 `getwchar` 的弱别名，行为完全一致。
