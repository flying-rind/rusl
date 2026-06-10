# setbuffer 函数规约

## 复杂度分级: Level 1

> musl libc `setbuffer` 实现 — GNU 扩展，为 FILE 流设置缓冲模式和自定义大小缓冲区。与 `setbuf` 的区别在于允许调用方指定缓冲区大小。

---

## 函数接口

```rust
use core::ffi::c_char;

extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: size_t);
```

[Visibility]: User — GNU / BSD 扩展函数（需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE`），用户程序可直接调用。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- `buf`: 若非 null，指向调用方提供的缓冲区，大小至少为 `size` 字节。该缓冲区在流关闭前必须保持有效
- `size`: 若 `buf` 非 null，指定缓冲区大小（字节数）。若 `buf` 为 null，此参数被忽略
- 根据 C 标准和实现惯例，`setbuffer` 的行为仅在流打开后、任何读写操作前被调用时是定义良好的
- 编译时需定义 `_GNU_SOURCE` 或 `_BSD_SOURCE` 以获取函数声明

**[Post-condition]:**
- 等价于 `setvbuf(f, buf, if buf.is_null() { _IONBF } else { _IOFBF }, size)`：
  - 若 `buf` 非 null：流 `f` 设为全缓冲模式（`_IOFBF`），缓冲区大小为 `size`
  - 若 `buf` 为 null：流 `f` 设为无缓冲模式（`_IONBF`），`size` 参数被忽略
- 流 `f` 的 `F_SVB` 标志被置位
- 无返回值（`void` → Rust 返回 `()`）

**[Error Behavior]:**
- 本函数不产生错误。`setvbuf` 不会因无效 `type_` 而失败（因传入的是合法值 `_IOFBF` 或 `_IONBF`）
- 不设置 errno

---

### 不变量

**[Invariant]:**
- `setbuffer(f, buf, size)` 始终等价于 `setvbuf(f, buf, buf.is_null() ? _IONBF : _IOFBF, size)`
- 调用后 `F_SVB` 标志必然置位

---

### 意图

带自定义缓冲区大小的缓冲设置接口。与 `setbuf` 的核心区别在于由调用方指定缓冲区大小而非使用默认的 `BUFSIZ`。

Rust 侧实现：
- 内部直接委托给 `setvbuf`，无需重复实现
- `buf` 参数使用 `*mut c_char` 与 C 的 `char *` 对应
- `size` 参数使用 `usize`（对应 C 的 `size_t`）
- 使用 `buf.is_null()` 判断是否传入缓冲区
- 因是 GNU 扩展，可放在 `#[cfg(feature = "gnu")]` 或类似的 feature gate 下

### 系统算法

```
setbuffer(f, buf, size):
  1. type_ = 若 buf.is_null() { _IONBF } 否则 { _IOFBF }
  2. setvbuf(f, buf, type_, size)
  3. 返回  // void 函数
```

时间复杂度 O(1)（取决于 `setvbuf` 的 O(1) 复杂度）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `_IOFBF` | 0 | `<stdio.h>` | 全缓冲模式 |
| `_IONBF` | 2 | `<stdio.h>` | 无缓冲模式 |

---

## 依赖图

```
setbuffer (Public, GNU extension)
  └── setvbuf (see setvbuf spec)
```

---

## [RELY]

- `setvbuf(f: *mut FILE, buf: *mut c_char, type_: c_int, size: size_t) -> c_int` — 底层的缓冲设置函数（见 `setvbuf` spec）
- 常量 `_IOFBF`、`_IONBF`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: size_t);`

本模块保证对外提供 ABI 兼容的 `setbuffer` 函数符号，行为符合 GNU/BSD 扩展语义。`setbuffer(f, buf, size)` 始终等价于 `setvbuf(f, buf, buf ? _IOFBF : _IONBF, size)`。
