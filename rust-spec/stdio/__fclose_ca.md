# __fclose_ca 函数规约

## 复杂度分级: Level 1

> musl libc 内部调用方分配 FILE（Caller-Allocated）的关闭操作实现。为调用方分配的 `FILE` 结构体提供关闭语义：仅调用 `f->close` 函数指针，不释放 `FILE` 内存。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __fclose_ca(f: *mut FILE) -> c_int;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。与 `__fopen_rb_ca` 配套使用，供调用方自行管理 `FILE` 结构体内存的场景。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针，其 `close` 函数指针已设置为有效的关闭函数（如 `__stdio_close`）
- `f` 指向的内存由调用方管理（不在堆上，或由调用方自行释放）

**[Post-condition]:**

**Case 1: 关闭成功**
- 底层文件描述符已关闭（通过 `f->close` 回调）
- `*mut FILE` 内存未被释放（调用方负责后续处理）
- 返回 `f->close(f)` 的返回值（通常为 `0`）

**Case 2: 关闭失败**
- 返回 `-1`，errno 由底层关闭函数设置

**[Error Behavior]:**
- 本函数自身不产生错误，仅转发 `f->close` 的返回值。若 `f->close` 失败，errno 由底层关闭操作设置。

---

### 不变量

**[Invariant]:**
- 本函数仅作转发调用（thin wrapper），不执行内存分配/释放
- `f` 指针在调用前后始终有效（由调用方管理其生命周期）

---

### 意图

关闭调用方分配的 `FILE` 流。与标准 `fclose` 的关键区别在于不释放 `FILE*` 内存（`f` 由调用方管理），仅调用 `f->close(f)` 关闭底层文件描述符。

Rust 侧实现：
- 保持为简单的 `unsafe extern "C"` 薄封装，直接转发调用 `(*f).close`
- 内部实现可考虑安全检查：断言 `f` 非空、`close` 函数指针非空（debug 模式）

---

### 系统算法

```
__fclose_ca(f):
  return (*f).close(f)
```

时间复杂度 O(1)（取决于 `f->close` 的具体实现）。

---

## 依赖图

```
__fclose_ca
  └─> (*f).close(f)    (see __stdio_close spec)
```

---

## [RELY]

- `(*f).close` — 流关闭函数指针（通常指向 `__stdio_close`），由调用方在构造 `FILE` 时设置

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __fclose_ca(f: *mut FILE) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号。行为和 C 实现完全一致：仅依次调用 `f->close(f)`，不释放 `f` 内存，不修改 errno（除非底层关闭操作设置）。
