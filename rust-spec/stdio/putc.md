# putc 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 宏兼容字符写入实现。`putc` 在 `<stdio.h>` 中通常被定义为宏以提供性能优化，但同时也需要作为函数存在以支持函数指针等场景。musl 提供 `putc` 函数实现和 `_IO_putc` 弱别名。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// putc: 用户可见函数（宏的备选函数实现）
extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int;

// _IO_putc: 弱别名，与 putc 共享同一实现
// weak_alias: _IO_putc 是 putc 的弱别名
extern "C" fn _IO_putc(c: c_int, f: *mut FILE) -> c_int;
```

[Visibility]:
- `putc`: User — `<stdio.h>` 标准库函数（宏的备选函数实现），用户程序可调用（通过函数指针或 `#undef putc` 后）。
- `_IO_putc`: Internal — 传统 `_IO_` 前缀历史兼容别名。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `c`: 要写入的字符（以 `c_int` 传递，内部转为 `unsigned char`）。
- `f`: 非空 `*mut FILE` 指针，指向已打开的写模式流。

**[Post-condition]:**
- **Case 1 成功写入**
  - 返回写入的字符（0-255 的 `c_int` 值）。
  - 字符已写入流。

- **Case 2 写入错误**
  - 返回 `EOF`（-1）。
  - FILE 流设置 `F_ERR` 标志。

**[Error Behavior]:**
- 写入错误时返回 `EOF`。

---

### 不变量

**[Invariant]:**
- 对于行缓冲流，遇到 `\n` 时缓冲区会被刷出。
- 成功写入时返回值始终在 0-255 范围内。
- `putc` 和 `_IO_putc` 行为完全一致。

---

### 意图

将字符 `c` 写入 FILE 流 `f`。通常 `<stdio.h>` 以宏形式内联展开为高效实现，但 musl 同时提供函数实现以支持函数指针等用例。

Rust 侧实现：
- 内部通过 `do_putc(c, f)` 完成实际写入，`do_putc` 定义于 `putc.h` 对应的 Rust 内部模块中。
- `putc` 和 `_IO_putc` 在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

### 系统算法

```
putc(c: c_int, f: *mut FILE) -> c_int:
  return do_putc(c, f)
```

参见 `putc.h` 对应的 Rust spec 中 `do_putc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
putc (Public)
  └── do_putc (内部 inline 函数, from "putc.rs" 内部模块)
        ├── putc_unlocked (内部宏/内联函数)
        │     └── __overflow
        ├── locking_putc (内部 static 函数)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── putc_unlocked
        └── __pthread_self

weak_alias(putc, _IO_putc)
```

---

## [RELY]

- `do_putc(c_int, *mut FILE)` — 内部字符写入函数（`putc.rs` 内部模块）。
- `__overflow(*mut FILE)` — 底层缓冲区刷出函数。
- `__lockfile(*mut FILE)` / `__unlockfile(*mut FILE)` — 文件锁函数。
- `__pthread_self()` — 线程自引用。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn putc(c: c_int, f: *mut FILE) -> c_int;`
  `extern "C" fn _IO_putc(c: c_int, f: *mut FILE) -> c_int;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 ISO C 标准 `putc` 语义。`_IO_putc` 与 `putc` 行为完全一致，为弱别名关系。
