# putwchar 函数规约

## 复杂度分级: Level 1

> musl libc 标准输出宽字符写入函数。将一个宽字符写入 `stdout`。

---

## 函数接口

```rust
use core::ffi::c_uint;

// putwchar — User, 标准库函数
extern "C" fn putwchar(c: c_uint /* wchar_t */) -> c_uint; // wint_t

// putwchar_unlocked — User (weak_alias -> putwchar)
extern "C" fn putwchar_unlocked(c: c_uint) -> c_uint;
```

[Visibility]: `putwchar` 是 `<wchar.h>` 标准库函数，对外导出。`putwchar_unlocked` 是 POSIX 免锁扩展，通过弱别名对外导出（在 musl 中与 `putwchar` 指向同一实现）。

Rust 侧实现策略：
- 直接委托给 `fputwc(c, stdout)`，实现极为简单
- `stdout` 通过全局访问器获取

---

### 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的宽字符（`wchar_t` 类型）
- `stdout` 已正确初始化并处于可写状态

**[Post-condition]:**
- **Case 1 成功写入宽字符**
  - 返回写入的宽字符值 `c`
  - 宽字符已转换为多字节序列并写入 `stdout`

- **Case 2 写入错误或编码错误**
  - 返回 `WEOF`
  - `stdout` 设置 `F_ERR` 标志

**[Error Behavior]:**
- 同 `fputwc`

---

### 不变量

无。本函数纯粹作为转发代理。

---

### 意图

将宽字符 `c` 写入标准输出流 `stdout`。等价于 `putwc(c, stdout)` 或 `fputwc(c, stdout)`。

---

### 系统算法

```
putwchar(c):
  return fputwc(c, stdout)
```

时间复杂度 O(1)（转发）。

---

## 依赖图

```
putwchar (Public)
  └── fputwc(c, stdout)  (see fputwc.c spec)

putwchar_unlocked (weak_alias)
  └── putwchar

stdout (全局变量, 来自 <stdio.h>)
```

---

## [RELY]

- `fputwc` — 宽字符写入函数 (见 `fputwc.md`)
- `stdout` — 标准输出 FILE 指针 (`<stdio.h>` 全局变量)

## [GUARANTEE]

Exported Interface:
  `extern "C" fn putwchar(c: c_uint) -> c_uint;`
  `extern "C" fn putwchar_unlocked(c: c_uint) -> c_uint;`

本模块保证对外提供上述 ABI 兼容的函数符号。`putwchar_unlocked` 为 `putwchar` 的弱别名，行为完全一致。
