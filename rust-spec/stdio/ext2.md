# ext2 函数规约 (stdio_ext.h 扩展函数 第二部分)

## 复杂度分级: Level 1

> musl libc GNU stdio 扩展函数实现（第二部分）。提供 `stdio_ext.h` 中声明的读缓冲区窥探函数和错误标志设置函数。所有函数直接访问 FILE 结构体内部字段。Rust 实现中，外部接口保持 ABI 兼容，内部字段访问使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 rusl 内部类型，定义于 stdio_impl 模块
// size_t 映射为 usize，与 C size_t ABI 兼容 (64 位平台为 u64)

// 1. __freadahead
unsafe extern "C" fn __freadahead(f: *mut FILE) -> usize;

// 2. __freadptr
unsafe extern "C" fn __freadptr(f: *mut FILE, sizep: *mut usize) -> *const c_char;

// 3. __freadptrinc
unsafe extern "C" fn __freadptrinc(f: *mut FILE, inc: usize);

// 4. __fseterr
unsafe extern "C" fn __fseterr(f: *mut FILE);
```

[Visibility]: 所有函数均声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数（User 可见）。这些函数直接访问 FILE 内部读缓冲区和标志字段，行为高度依赖 FILE 内存布局。

**注意**: `__freadptr` 和 `__freadptrinc` 提供零拷贝读取能力——允许调用者直接访问 FILE 内部缓冲区。这种模式要求调用者遵守严格的调用规约（见下方注意事项）。

---

## 各函数规约

### 1. `__freadahead`

```rust
unsafe extern "C" fn __freadahead(f: *mut FILE) -> usize;
```

**[Intent]**: 返回 FILE 流读缓冲区中还可读取的字节数（预读字节数）。允许调用者在不实际调用 `fread`/`fgetc` 前窥探缓冲区可用数据量。配合 `__freadptr`/`__freadptrinc` 实现零拷贝读取。

**[Pre-condition]**: `f` 非空 `*mut FILE` 指针。

**[Post-condition]**: 返回 `rend - rpos`（若 `rend != null`），否则返回 `0`。

**[System Algorithm]**:
```
__freadahead(f):
  若 f.rend != null: f.rend - f.rpos
  否则: 0
```

---

### 2. `__freadptr`

```rust
unsafe extern "C" fn __freadptr(f: *mut FILE, sizep: *mut usize) -> *const c_char;
```

**[Intent]**: 返回指向 FILE 流读缓冲区当前位置的指针，并通过 `*sizep` 返回可读字节数。若读缓冲区为空（`rpos == rend`），返回 NULL 且不修改 `*sizep`。

**用途**: 允许调用者直接访问 FILE 内部缓冲区，实现零拷贝读取。

**[Pre-condition]**:
- `f`: 非空 `*mut FILE` 指针。
- `sizep`: 非空 `*mut usize` 指针，指向可写的 `usize` 变量。

**[Post-condition]**:
- **Case 1 读缓冲区有数据**:
  - `*sizep` 被设置为 `f.rend - f.rpos`。
  - 返回指向 `f.rpos` 的 `*const c_char` 指针。
- **Case 2 读缓冲区无可用数据**:
  - `*sizep` 不变。
  - 返回 `core::ptr::null()`。

**[System Algorithm]**:
```
__freadptr(f, sizep):
  若 f.rpos == f.rend:  // 缓冲区空或被清空
    返回 null
  *sizep = f.rend - f.rpos
  返回 f.rpos as *const c_char
```

---

### 3. `__freadptrinc`

```rust
unsafe extern "C" fn __freadptrinc(f: *mut FILE, inc: usize);
```

**[Intent]**: 将 FILE 流读缓冲区的读指针推进 `inc` 字节。与 `__freadptr` 配合使用实现零拷贝读取：先用 `__freadptr` 获取缓冲区指针并直接访问数据，再用 `__freadptrinc` 标记已消费。

**[Pre-condition]**:
- `f`: 非空 `*mut FILE` 指针。
- `inc`: 推进量，应不大于 `__freadahead(f)` 返回的可用字节数（**musl 不检查，调用者负责**）。

**[Post-condition]**: `f.rpos` 增加 `inc`。已推进的数据不再可通过 `__freadptr` 访问。无返回值。

**[System Algorithm]**:
```
__freadptrinc(f, inc):
  f.rpos += inc
```

**注意事项**:
- musl 实现不做边界检查。若 `inc` 超出 `rend - rpos`，读指针越过缓冲区末尾，后续行为未定义。Rust 侧在 debug 模式下可添加 `debug_assert!` 进行开发期检查。
- 在调用 `__freadptr` 和 `__freadptrinc` 之间，不得对同一流调用任何其他 stdio 函数（可能使返回的指针失效或改变缓冲区状态）。

---

### 4. `__fseterr`

```rust
unsafe extern "C" fn __fseterr(f: *mut FILE);
```

**[Intent]**: 直接设置 FILE 流的错误标志位（`F_ERR`）。与 `ferror(f)`（查询）互补，允许调用者手动将流标记为错误状态。

**[Pre-condition]**: `f` 非空 `*mut FILE` 指针。

**[Post-condition]**:
- `f.flags` 的 `F_ERR` 位被设置。
- 之后 `ferror(f)` 将返回非零值。
- 后续 I/O 操作将受错误状态影响。

**[System Algorithm]**:
```
__fseterr(f):
  f.flags |= F_ERR
```

---

## 不变量

**[Invariant]:**
- `__freadahead` 和 `__freadptr` 为纯查询函数，不修改流状态。
- `__freadptrinc` 仅推进读指针，不修改其他状态。
- `__fseterr` 仅设置 `F_ERR` 标志位，不修改其他状态。
- 所有函数为 O(1) 时间复杂度。

---

## 意图

提供 GNU `stdio_ext.h` 扩展接口的第二部分：读缓冲区窥探与零拷贝读取能力，以及手动错误标志设置。

Rust 侧实现：
- 所有函数使用 `unsafe extern "C"` 声明，保持 ABI 兼容。
- 内部 FILE 结构体字段访问通过裸指针解引用（`(*f).rpos`）在 unsafe 块中进行。
- `__freadahead` 返回值为 `usize`，在 64 位平台上与 C `size_t`（8 字节）ABI 兼容。
- `__freadptr` 返回 `*const c_char`（即 `*const u8`/`*const i8`），与 C `const char *` ABI 兼容。
- `__freadptrinc` 的指针算术使用 `f.rpos = f.rpos.add(inc)` 或在 Rust 中通过 `usize` 偏移量加到原始指针上。
- `__fseterr` 的按位或赋值通过 `(*f).flags |= F_ERR` 实现。
- `F_ERR` 使用 Rust 常量或 `bitflags` crate（`no_std` 兼容）定义。

---

## 零拷贝读取模式示例

```
// 传统读取 (fread — 有拷贝):
let mut buf = [0u8; 1024];
let n = fread(buf.as_mut_ptr().cast(), 1, 1024, f);

// 零拷贝读取 (__freadptr + __freadptrinc):
let mut n: usize = 0;
let p = __freadptr(f, &mut n);
// p 指向 FILE 内部读缓冲区，n 为可用字节数
process_data(p, n);             // 直接访问，无数据拷贝
__freadptrinc(f, n);            // 推进读指针
```

**关键规则**:
1. 在 `__freadptr` 和 `__freadptrinc` 之间不得对同一流调用任何其他 stdio 函数。
2. `__freadptrinc` 的 `inc` 不得超出 `__freadahead` 返回的可用字节数。
3. `__freadptr` 返回的指针仅在流未被进一步操作时有效。

---

## 依赖图

```
__freadahead
  └── FILE.rend, FILE.rpos

__freadptr
  └── FILE.rpos, FILE.rend

__freadptrinc
  └── FILE.rpos

__fseterr
  └── FILE.flags (F_ERR)
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `FILE` 类型 (flags, rend, rpos 字段) — rusl stdio_impl 模块定义。
- 内部 `F_ERR` 标志位常量 — rusl stdio_impl 模块定义。

## [GUARANTEE]

Exported Interface:
  ```
  unsafe extern "C" fn __freadahead(f: *mut FILE) -> usize;
  unsafe extern "C" fn __freadptr(f: *mut FILE, sizep: *mut usize) -> *const c_char;
  unsafe extern "C" fn __freadptrinc(f: *mut FILE, inc: usize);
  unsafe extern "C" fn __fseterr(f: *mut FILE);
  ```

本模块保证对外提供上述 4 个 ABI 兼容的函数符号。所有函数行为符合 GNU `stdio_ext.h` 扩展语义。`__freadptr` 和 `__freadptrinc` 的零拷贝读取语义与原 musl 实现完全一致。这些函数高度依赖 FILE 结构体的内部字段布局，必须与原 musl 实现的 FILE 结构体保持严格一致。
