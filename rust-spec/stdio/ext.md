# ext 函数规约 (stdio_ext.h 扩展函数)

## 复杂度分级: Level 1

> musl libc GNU stdio 扩展函数实现。提供 `stdio_ext.h` 中声明的流查询与操作函数。所有函数直接访问 FILE 结构体内部字段。Rust 实现中，外部接口保持 ABI 兼容，内部字段访问使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

// FILE 为 rusl 内部类型，定义于 stdio_impl 模块
// size_t 映射为 usize，但需确保与 C size_t 的 ABI 兼容 (64 位平台为 u64)

// 1. _flushlbf
unsafe extern "C" fn _flushlbf();

// 2. __fsetlocking
unsafe extern "C" fn __fsetlocking(f: *mut FILE, type_: c_int) -> c_int;

// 3. __fwriting
unsafe extern "C" fn __fwriting(f: *mut FILE) -> c_int;

// 4. __freading
unsafe extern "C" fn __freading(f: *mut FILE) -> c_int;

// 5. __freadable
unsafe extern "C" fn __freadable(f: *mut FILE) -> c_int;

// 6. __fwritable
unsafe extern "C" fn __fwritable(f: *mut FILE) -> c_int;

// 7. __flbf
unsafe extern "C" fn __flbf(f: *mut FILE) -> c_int;

// 8. __fbufsize
unsafe extern "C" fn __fbufsize(f: *mut FILE) -> usize;

// 9. __fpending
unsafe extern "C" fn __fpending(f: *mut FILE) -> usize;

// 10. __fpurge / fpurge (weak_alias)
unsafe extern "C" fn __fpurge(f: *mut FILE) -> c_int;
// weak_alias: fpurge 是 __fpurge 的弱别名，共享同一实现
unsafe extern "C" fn fpurge(f: *mut FILE) -> c_int;
```

[Visibility]: 所有函数均声明于 `<stdio_ext.h>`，需定义 `_GNU_SOURCE`。GNU stdio 扩展函数。由于这些函数访问 FILE 内部结构体字段，其行为高度依赖 FILE 结构体的内存布局——必须与原 musl 实现的 FILE 结构体布局保持完全一致。

**注意**: `_flushlbf`、`__fsetlocking`、`__fwriting` 等以 `_` 或 `__` 开头的函数虽前缀似内部符号，但在 `<stdio_ext.h>` 中公开声明，属于 User 可见的 GNU 扩展 API，必须保持 ABI 兼容。

---

## 各函数规约

### 1. `_flushlbf`

```rust
unsafe extern "C" fn _flushlbf();
```

**[Intent]**: 刷新所有行缓冲（line-buffered）的 FILE 流。通过调用 `fflush(core::ptr::null_mut())` 刷新所有打开的输出流。

**[Pre-condition]**: 无。

**[Post-condition]**: 所有打开的行缓冲 FILE 流中挂起的数据被刷新到底层文件描述符。无返回值。

**[System Algorithm]**:
```
_flushlbf():
  fflush(NULL)  // 刷新所有打开流
```

---

### 2. `__fsetlocking`

```rust
unsafe extern "C" fn __fsetlocking(f: *mut FILE, type_: c_int) -> c_int;
```

**[Intent]**: 设置 FILE 流的锁定行为。musl 的实现始终返回 `0`（锁定行为未改变），因为 musl 不使用可切换的内部锁策略。

**参数 `type_`**:
- `FSETLOCKING_INTERNAL` (0) — 设置隐式锁定
- `FSETLOCKING_BYCALLER` (1) — 调用者自行锁定
- `FSETLOCKING_QUERY` (2) — 查询当前状态

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 始终返回 `0`（musl 简化实现）。

**[System Algorithm]**: `return 0`

---

### 3. `__fwriting`

```rust
unsafe extern "C" fn __fwriting(f: *mut FILE) -> c_int;
```

**[Intent]**: 查询流是否处于"正在写入"状态。检查 `F_NORD` 标志或写缓冲区挂起数据。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回非零值表示流处于写模式，`0` 表示否。

**[System Algorithm]**:
```
__fwriting(f):
  (f.flags & F_NORD) != 0 || f.wend != null
```

---

### 4. `__freading`

```rust
unsafe extern "C" fn __freading(f: *mut FILE) -> c_int;
```

**[Intent]**: 查询流是否处于"正在读取"状态。检查 `F_NOWR` 标志或读缓冲区可用数据。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回非零值表示流处于读模式，`0` 表示否。

**[System Algorithm]**:
```
__freading(f):
  (f.flags & F_NOWR) != 0 || f.rend != null
```

---

### 5. `__freadable`

```rust
unsafe extern "C" fn __freadable(f: *mut FILE) -> c_int;
```

**[Intent]**: 查询流是否可读（`F_NORD` 标志未设置）。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回非零值表示流可读。

**[System Algorithm]**:
```
__freadable(f):
  (f.flags & F_NORD) == 0
```

---

### 6. `__fwritable`

```rust
unsafe extern "C" fn __fwritable(f: *mut FILE) -> c_int;
```

**[Intent]**: 查询流是否可写（`F_NOWR` 标志未设置）。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回非零值表示流可写。

**[System Algorithm]**:
```
__fwritable(f):
  (f.flags & F_NOWR) == 0
```

---

### 7. `__flbf`

```rust
unsafe extern "C" fn __flbf(f: *mut FILE) -> c_int;
```

**[Intent]**: 查询流是否使用行缓冲模式（`lbf >= 0`）。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回非零值表示行缓冲，`0` 表示全缓冲或无缓冲。

**[System Algorithm]**:
```
__flbf(f):
  f.lbf >= 0
```

---

### 8. `__fbufsize`

```rust
unsafe extern "C" fn __fbufsize(f: *mut FILE) -> usize;
```

**[Intent]**: 返回流的缓冲区大小。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回 `f.buf_size`。

**[System Algorithm]**:
```
__fbufsize(f):
  f.buf_size
```

---

### 9. `__fpending`

```rust
unsafe extern "C" fn __fpending(f: *mut FILE) -> usize;
```

**[Intent]**: 返回写缓冲区中待写入的字节数。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**: 返回 `wpos - wbase`（若有写缓冲区）或 `0`。

**[System Algorithm]**:
```
__fpending(f):
  若 f.wend != null: f.wpos - f.wbase
  否则: 0
```

---

### 10. `__fpurge` / `fpurge`

```rust
unsafe extern "C" fn __fpurge(f: *mut FILE) -> c_int;
// weak_alias: fpurge 与 __fpurge 行为完全一致，为弱别名关系
unsafe extern "C" fn fpurge(f: *mut FILE) -> c_int;
```

**[Intent]**: 清空 FILE 流的所有内部缓冲区（读和写），丢弃所有未处理数据。将 `wpos`、`wbase`、`wend`、`rpos`、`rend` 全部置零。

**[Pre-condition]**: `f` 非空 FILE 指针。

**[Post-condition]**:
- 读缓冲区被清空（`rpos = rend = null`）。
- 写缓冲区被清空（`wpos = wbase = wend = null`）。
- 缓冲区中所有未处理数据已丢弃（不可恢复）。
- 始终返回 `0`。

**[System Algorithm]**:
```
__fpurge(f):
  f.wpos = f.wbase = f.wend = null
  f.rpos = f.rend = null
  return 0
```

---

## 不变量

**[Invariant]:**
- 所有查询函数（`__fwriting`、`__freading`、`__freadable`、`__fwritable`、`__flbf`）为纯函数，不修改流的任何状态。
- `__fbufsize`、`__fpending`、`__freadahead` 为纯查询，无副作用。
- `__fpurge` / `fpurge` 清空缓冲区，数据丢弃后不可恢复，始终返回 `0`。
- `__fsetlocking` 在 musl 中为无操作，始终返回 `0`。
- `_flushlbf` 触发所有行缓冲流的 fflush，可能修改流状态和 errno。
- `fpurge` 是 `__fpurge` 的弱别名（weak_alias），二者行为完全一致，共享同一函数体。

---

## 意图

提供 GNU `stdio_ext.h` 扩展接口，允许程序查询和控制 FILE 流的内部状态。这些函数在 musl 中直接访问 FILE 结构体的内部字段。

Rust 侧实现：
- 所有函数使用 `unsafe extern "C"` 声明，保持 ABI 兼容。
- 内部 FILE 结构体字段访问通过 Rust 安全字段访问（`(*f).flags`、`(*f).wend` 等），使用 `NonNull<FILE>` 或 `&mut FILE` 安全引用代替裸指针操作。
- `F_NORD`、`F_NOWR`、`F_ERR` 等标志位使用 Rust 常量定义或 `bitflags` crate（支持 `no_std`）。
- `_flushlbf` 委托给内部 `fflush` 函数。
- `__fpurge` 和 `fpurge` 在 Rust 中共享同一函数体——可通过如下方式实现：
  - 两个独立的 `#[no_mangle] unsafe extern "C" fn` 调用同一个内部 `unsafe fn fpurge_impl(f: *mut FILE) -> c_int` 函数。
  - 或通过 `#[export_name]` 属性将 `fpurge` 导出为 `__fpurge` 的别名（若链接器支持）。
- 所有函数操作极简（O(1)），无需额外内存分配。

---

## 依赖图

```
_flushlbf
  └── [Internal] fflush(NULL)

__fsetlocking
  └── 无实质依赖 (直接返回 0)

__fwriting
  └── FILE.flags (F_NORD), FILE.wend

__freading
  └── FILE.flags (F_NOWR), FILE.rend

__freadable
  └── FILE.flags (F_NORD)

__fwritable
  └── FILE.flags (F_NOWR)

__flbf
  └── FILE.lbf

__fbufsize
  └── FILE.buf_size

__fpending
  └── FILE.wend, FILE.wpos, FILE.wbase

__fpurge / fpurge
  └── FILE.wpos, FILE.wbase, FILE.wend, FILE.rpos, FILE.rend
```

---

## 标志位语义速查

| 字段/表达式 | 含义 |
|-------------|------|
| `f.flags & F_NORD` | 流不可读（已进入写模式） |
| `f.flags & F_NOWR` | 流不可写（已进入读模式） |
| `f.flags & F_ERR` | 流出错 |
| `f.flags & F_EOF` | 流遇到文件尾 |
| `f.lbf >= 0` | 行缓冲模式（`lbf='\n'` 行缓冲, `lbf=EOF` 全缓冲, `lbf=0` 无缓冲） |
| `f.wpos - f.wbase` | 写缓冲区中待写入字节数 |
| `f.rend - f.rpos` | 读缓冲区中剩余可读字节数 |
| `f.buf_size` | 缓冲区总容量 |

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `FILE` 类型 (flags, wend, rend, wpos, wbase, rpos, lbf, buf_size 字段) — rusl stdio_impl 模块定义。
- 内部 `F_NORD`, `F_NOWR` 标志位常量 — rusl stdio_impl 模块定义。
- 内部 `fflush` — 仅 `_flushlbf` 依赖。
- 无其他外部依赖。所有函数 O(1)，直接字段访问。

## [GUARANTEE]

Exported Interface:
  ```
  unsafe extern "C" fn _flushlbf();
  unsafe extern "C" fn __fsetlocking(f: *mut FILE, type_: c_int) -> c_int;
  unsafe extern "C" fn __fwriting(f: *mut FILE) -> c_int;
  unsafe extern "C" fn __freading(f: *mut FILE) -> c_int;
  unsafe extern "C" fn __freadable(f: *mut FILE) -> c_int;
  unsafe extern "C" fn __fwritable(f: *mut FILE) -> c_int;
  unsafe extern "C" fn __flbf(f: *mut FILE) -> c_int;
  unsafe extern "C" fn __fbufsize(f: *mut FILE) -> usize;
  unsafe extern "C" fn __fpending(f: *mut FILE) -> usize;
  unsafe extern "C" fn __fpurge(f: *mut FILE) -> c_int;
  unsafe extern "C" fn fpurge(f: *mut FILE) -> c_int;   // weak_alias of __fpurge
  ```

本模块保证对外提供上述 11 个 ABI 兼容的函数符号。所有函数行为符合 GNU `stdio_ext.h` 扩展语义。`fpurge` 和 `__fpurge` 为弱别名关系，行为完全一致。这些函数高度依赖 FILE 结构体的内部字段布局，必须与原 musl 实现的 FILE 结构体保持严格一致。
