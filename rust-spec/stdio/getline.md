# getline 函数规约

## 复杂度分级: Level 1

> musl libc 标准换行符分隔的动态行读取实现。`getline` 是 `getdelim` 的特化版本，固定以 `\n` 为分隔符。

---

## 函数接口

```rust
use core::ffi::c_char;

// FILE 为 opaque 类型，定义于 rusl-internal 模块

extern "C" fn getline(s: *mut *mut c_char, n: *mut usize, f: *mut FILE) -> isize;
```

[Visibility]: User — POSIX.1-2008 标准函数，用户程序包含 `<stdio.h>` 即可调用。必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。返回值为 `isize`（对应 C 的 `ssize_t`）。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向 `*mut c_char` 变量（该变量可为 `null_mut()` 或指向已有缓冲区）。
- `n`: 非空指针，指向 `usize` 变量（指示当前缓冲区大小）。
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。

**[Post-condition]:**
- **Case 1 成功读取一行（以换行符结尾）**
  - 返回读取的字符数（**包括** `\n`，不包括结尾的 `\0`）。
  - `*s` 指向包含完整行 + `\0` 的动态分配缓冲区。
  - `*n` 更新为当前缓冲区大小。

- **Case 2 成功读取到文件末尾（无换行符，如文件末尾缺少换行）**
  - 返回读取的字符数（不包括 `\0`）。
  - `*s` 指向包含数据 + `\0` 的缓冲区。
  - FILE 流设置 `F_EOF` 标志。

- **Case 3 参数无效 (`s` 或 `n` 为 `null_mut()`)**
  - 返回 -1，`errno = EINVAL`。

- **Case 4 内存不足或读取错误**
  - 返回 -1。

**[Error Behavior]:**
- 参数无效返回 -1 并设置 `errno = EINVAL`。其他错误由 `getdelim` 处理。

---

### 不变量

**[Invariant]:**
- 等价于 `getdelim(s, n, '\n', f)`。
- 返回值包含 `\n`（除非到达 EOF 且最后一个字符不是 `\n`）。

---

### 意图

从 FILE 流 `f` 中读取以 `\n`（换行符）结尾的一行数据。等价于 `getdelim(s, n, '\n', f)`。详细信息参见 `getdelim` 规约。

Rust 侧实现：
- 直接委托给 `getdelim(s, n, '\n' as c_int, f)`。
- 无需独立的复杂逻辑，仅是对 `getdelim` 的封装调用。

### 系统算法

```
getline(s: *mut *mut c_char, n: *mut usize, f: *mut FILE) -> isize:
  return getdelim(s, n, '\n' as c_int, f)
```

时间复杂度同 `getdelim`：O(n)，n 为实际读取的字符数。

---

## 依赖图

```
getline (Public)
  └── getdelim (see getdelim spec)
        ├── FLOCK / FUNLOCK
        ├── memchr / ptr::copy_nonoverlapping
        ├── realloc / alloc
        ├── getc_unlocked
        ├── feof
        └── set_errno / EINVAL / ENOMEM
```

---

## [RELY]

- `getdelim(*mut *mut c_char, *mut usize, c_int, *mut FILE)` — 通用分隔符行读取函数。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getline(s: *mut *mut c_char, n: *mut usize, f: *mut FILE) -> isize;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为符合 POSIX.1-2008 `getline` 语义。
