# setbuf 函数规约

## 复杂度分级: Level 1

> musl libc `setbuf` 实现 — 为 FILE 流设置缓冲模式和缓冲区。`setbuf` 是 `setvbuf` 的简化包装：当 `buf` 为 NULL 时设为无缓冲模式，否则设为全缓冲模式并使用 BUFSIZ 作为缓冲区大小。

---

## 函数接口

```rust
use core::ffi::c_char;

extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char);
```

[Visibility]: User — `<stdio.h>` 标准库函数，用户程序可直接调用。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- `buf`: 若非 null，指向调用方提供的缓冲区，大小至少为 `BUFSIZ` 字节。该缓冲区在流关闭前必须保持有效
- 根据 C 标准，`setbuf` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的

**[Post-condition]:**
- 等价于 `setvbuf(f, buf, if buf.is_null() { _IONBF } else { _IOFBF }, BUFSIZ)`：
  - 若 `buf` 非 null：流 `f` 设为全缓冲模式（`_IOFBF`），缓冲区大小为 `BUFSIZ`（1024）
  - 若 `buf` 为 null：流 `f` 设为无缓冲模式（`_IONBF`）
- 流 `f` 的 `F_SVB` 标志被置位
- 无返回值（`void` → Rust 返回 `()`）

**[Error Behavior]:**
- 本函数不产生错误。`setvbuf` 不会因无效 `type_` 而失败（因传入的是合法值 `_IOFBF` 或 `_IONBF`）
- 不设置 errno

---

### 不变量

**[Invariant]:**
- `setbuf(f, buf)` 始终等价于 `setvbuf(f, buf, buf.is_null() ? _IONBF : _IOFBF, BUFSIZ)`
- 调用后 `F_SVB` 标志必然置位

---

### 意图

简化缓冲区设置接口。提供与 `setvbuf` 相同的底层功能，但接口更简洁：仅需传递流指针和缓冲区指针，缓冲模式和大小自动推导。

Rust 侧实现：
- 内部直接委托给 `setvbuf`，无需重复实现
- `buf` 参数使用 `*mut c_char` 与 C 的 `char *` 对应
- 使用 `buf.is_null()` 判断是否传入缓冲区
- `BUFSIZ` 定义为常量 `const BUFSIZ: usize = 1024;`

### 系统算法

```
setbuf(f, buf):
  1. type_ = 若 buf.is_null() { _IONBF } 否则 { _IOFBF }
  2. setvbuf(f, buf, type_, BUFSIZ)
  3. 返回  // void 函数
```

时间复杂度 O(1)（取决于 `setvbuf` 的 O(1) 复杂度）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IOFBF` | 0 | `<stdio.h>` | 全缓冲模式 |
| `_IONBF` | 2 | `<stdio.h>` | 无缓冲模式 |
| `BUFSIZ` | 1024 | `<stdio.h>` | 默认缓冲区大小 |

---

## 依赖图

```
setbuf (Public)
  └── setvbuf (see setvbuf spec)
```

---

## [RELY]

- `setvbuf(f: *mut FILE, buf: *mut c_char, type_: c_int, size: size_t) -> c_int` — 底层的缓冲设置函数（见 `setvbuf` spec）
- 常量 `_IOFBF`、`_IONBF`、`BUFSIZ`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char);`

本模块保证对外提供 ABI 兼容的 `setbuf` 函数符号，行为符合 POSIX/C11 标准定义。`setbuf(f, buf)` 始终等价于 `setvbuf(f, buf, buf ? _IOFBF : _IONBF, BUFSIZ)`。
