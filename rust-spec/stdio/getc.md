# getc 函数规约

## 复杂度分级: Level 1

> musl libc 标准 IO 宏兼容字符读取实现。`getc` 在 `<stdio.h>` 中通常被定义为宏以提供性能优化，但同时也需要作为函数存在以支持函数指针等场景。musl 提供 `getc` 函数实现和 `_IO_getc` 弱别名。

---

## 函数接口

```rust
use core::ffi::c_int;

// FILE 为 opaque 类型，定义于 rusl-internal 模块

// getc: 用户可见函数（宏的备选函数实现）
extern "C" fn getc(f: *mut FILE) -> c_int;

// _IO_getc: 弱别名，与 getc 共享同一实现
// weak_alias: _IO_getc 是 getc 的弱别名
extern "C" fn _IO_getc(f: *mut FILE) -> c_int;
```

[Visibility]:
- `getc`: User — `<stdio.h>` 标准库函数（宏的备选函数实现），用户程序可调用（通过函数指针或 `#undef getc` 后）。
- `_IO_getc`: Internal — 传统 `_IO_` 前缀历史兼容别名，供 glibc 兼容代码使用（如 libstdc++）。

两者均必须保持 ABI 兼容：`extern "C"` 导出，参数类型布局与原 C 接口一致。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 非空 `*mut FILE` 指针，指向已打开的读模式流。

**[Post-condition]:**
- **Case 1 成功读取字符**
  - 返回读取到的字符（0-255 的 `c_int` 值）。
  - FILE 流位置前进一个字符。

- **Case 2 到达文件末尾**
  - 返回 `EOF`（-1）。
  - FILE 流设置 `F_EOF` 标志。

- **Case 3 读取错误**
  - 返回 `EOF`。
  - FILE 流设置 `F_ERR` 标志。

**[Error Behavior]:**
- 到达 EOF 或读取错误时返回 `EOF`。调用者需通过 `feof(f)` / `ferror(f)` 区分原因。

---

### 不变量

**[Invariant]:**
- 成功读取时返回值始终在 0-255 范围内。
- `getc` 和 `_IO_getc` 行为完全一致，返回相同结果。

---

### 意图

从 FILE 流 `f` 中读取一个字符。通常 `<stdio.h>` 以宏形式内联展开为高效实现，但 musl 同时提供函数实现以支持：
- 通过函数指针调用 `getc`。
- `#undef getc` 后使用真实函数。
- `_IO_getc` 别名引用（libstdc++ 等使用）。

Rust 侧实现：
- 内部通过 `do_getc(f)` 完成实际读取，`do_getc` 定义于 `getc.h` 对应的 Rust 内部模块中。
- `getc` 和 `_IO_getc` 在 Rust 侧通过复制函数体实现，或使用 linker 脚本实现真正的弱符号别名。

### 系统算法

```
getc(f: *mut FILE) -> c_int:
  return do_getc(f)
```

参见 `getc.h` 对应的 Rust spec 中 `do_getc` 的完整算法描述。

时间复杂度 O(1)（不含底层 I/O）。

---

## 依赖图

```
getc (Public)
  └── do_getc (内部 inline 函数, from "getc.rs" 内部模块)
        ├── getc_unlocked (内部宏/内联函数)
        │     └── __uflow
        ├── locking_getc (内部 static 函数)
        │     ├── __lockfile / atomic CAS / __wake
        │     └── getc_unlocked
        └── __pthread_self

weak_alias(getc, _IO_getc)
```

---

## [RELY]

- `do_getc(*mut FILE)` — 内部字符读取函数（`getc.rs` 内部模块）。
- `__uflow(*mut FILE)` — 底层缓冲区填充函数。
- `__lockfile(*mut FILE)` / `__unlockfile(*mut FILE)` — 文件锁函数。
- `__pthread_self()` — 线程自引用。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn getc(f: *mut FILE) -> c_int;`
  `extern "C" fn _IO_getc(f: *mut FILE) -> c_int;`

本模块保证对外提供上述两个 ABI 兼容的函数符号，行为符合 ISO C 标准 `getc` 语义。`_IO_getc` 与 `getc` 行为完全一致，为弱别名关系。
