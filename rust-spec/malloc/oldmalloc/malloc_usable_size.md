# malloc_usable_size.rs 规约

> **对应 C spec**: `src/malloc/oldmalloc/spec/malloc_usable_size.md`
> **对应 C 源文件**: `src/malloc/oldmalloc/malloc_usable_size.c`
> **所属 crate**: `rusl` (`#![no_std]`)
> **模块路径**: `rusl::malloc::oldmalloc::malloc_usable_size`

---

## 依赖图

```
malloc_usable_size  (对外导出, extern "C")
  ├── Chunk          (内部结构体) → 定义于 malloc_impl.rs (oldmalloc 内嵌模块)
  ├── OVERHEAD       (内部常量) → 定义于 malloc_impl.rs
  ├── chunk_size()   (内部辅助函数) → 定义于 malloc_impl.rs
  ├── mem_to_chunk() (内部辅助函数) → 定义于 malloc_impl.rs
  └── realloc         (依赖符号) → rusl::malloc::oldmalloc::realloc (循环引用)

注意: C 原版中的 __realloc_dep 链接器级依赖注入技巧在 Rust 中省略。
      在 Rust 的 crate 编译模型下，同一 crate 内的跨模块循环引用由编译器自然解析，
      不需要 ELF 符号引用级别的链接器 hack。
```

---

## 内部类型与宏 (不对外导出)

### `struct Chunk`
[Visibility]: Internal (不导出) — 模块私有，仅 oldmalloc 内部模块间共享 (`pub(crate)`)

```rust
#[repr(C)]
pub(crate) struct Chunk {
    pub psize: usize,
    pub csize: usize,
    pub next: *mut Chunk,
    pub prev: *mut Chunk,
}
```

**设计说明**: 使用 `#[repr(C)]` 确保内存布局与 C 的 `struct chunk` 完全一致。
`malloc`/`free`/`realloc`/`malloc_usable_size` 均依赖此结构体在用户数据区前的固定偏移位置读写 chunk 元数据，
因此其字段顺序、对齐方式必须与 C ABI 兼容。

- `psize`: 前一个物理相邻 chunk 的大小（低 1 位用作 `C_INUSE` 标志）
- `csize`: 当前 chunk 的大小（低 1 位用作 `C_INUSE` 标志或 mmap 区分标志）
- `next`, `prev`: 空闲链表指针（仅当 chunk 在 bin 中时有效；已分配 chunk 中此区域与用户数据区重叠）

### `OVERHEAD` (常量)
[Visibility]: Internal — `pub(crate)`

```rust
pub(crate) const OVERHEAD: usize = 2 * core::mem::size_of::<usize>();
```

单个 chunk 的元数据开销，即 `psize + csize` 两个 `usize` 字段的大小（不含 `next`/`prev`）。
- 64 位系统: `OVERHEAD == 16`
- 32 位系统: `OVERHEAD == 8`

**Rust 设计**: 使用 `const` 常量替代 C 宏，类型安全且支持编译期求值。
`core::mem::size_of::<usize>()` 在 `no_std` 环境中可用。

### `C_INUSE` (常量)
[Visibility]: Internal — `pub(crate)`

```rust
pub(crate) const C_INUSE: usize = 1;
```

标志位常量。存储在 chunk 大小字段（`csize`/`psize`）的最低有效位，指示 chunk 是否正在使用中。

---

## 内部辅助函数 (不对外导出)

### `chunk_size(c)`
[Visibility]: Internal — 模块私有或 `pub(crate)`

```rust
/// 读取 chunk `c` 的有效大小，清除 C_INUSE 标志位。
///
/// # Safety
/// `c` 必须指向一个有效的 `Chunk` 实例。
#[inline(always)]
pub(crate) unsafe fn chunk_size(c: *const Chunk) -> usize {
    unsafe { (*c).csize & !C_INUSE }
}
```

**语义**: 等价于 C 的 `CHUNK_SIZE(c)` 宏。通过 `& !1`（即 `& !C_INUSE`）清除 `csize` 的最低标志位，返回纯大小值。

### `mem_to_chunk(p)`
[Visibility]: Internal — 模块私有或 `pub(crate)`

```rust
/// 将用户可见的指针 `p` 转换回内部 `Chunk` 指针。
///
/// # Safety
/// `p` 必须是由 `malloc`/`calloc`/`realloc`/`aligned_alloc` 返回的有效堆指针，
/// 或者为由 `BIN_TO_CHUNK` 计算的哨兵地址。
#[inline(always)]
pub(crate) unsafe fn mem_to_chunk(p: *mut core::ffi::c_void) -> *mut Chunk {
    unsafe { (p as *mut u8).sub(OVERHEAD) as *mut Chunk }
}
```

**语义**: 等价于 C 的 `MEM_TO_CHUNK(p)` 宏。用户指针指向 chunk 头部之后 `OVERHEAD` 字节处的数据区起始位置，
通过减去 `OVERHEAD` 偏移获得 chunk 元数据指针。使用 `*mut u8::sub()` 进行字节级指针运算，避免依赖 unstable 的 `byte_offset`。

---

## `malloc_usable_size` (对外导出)

```rust
extern "C" fn malloc_usable_size(p: *mut core::ffi::c_void) -> usize;
```

[Visibility]: External (导出) — GNU 扩展，用户程序可直接调用。使用 `extern "C"` 确保 C ABI 兼容。

### 功能描述

返回通过 `malloc` / `calloc` / `realloc` / `aligned_alloc` 分配的堆内存块的实际可用字节数。

返回值是 `malloc` 族函数在内部为该分配请求实际预留的内存大小——该值不小于原始 `malloc(size)` 调用时传入的 `size` 参数，
但可能因内部对齐与 chunk 开销策略而略大。用户可将该返回值作为上限，安全地访问该内存。

### 前置条件

- 若 `p != null`，则 `p` 必须是由同一分配器实例（rusl 的 `malloc` / `calloc` / `realloc` / `aligned_alloc`）返回且尚未被 `free` 的有效堆指针。
- 不得对栈变量、全局变量、`mmap` 直接返回的指针或已 `free` 的指针调用本函数，否则行为未定义。

### 后置条件

- **Case 1** (`p.is_null()`): 返回 `0`。不修改任何全局或堆状态。
- **Case 2** (`!p.is_null()`): 返回 `chunk_size(mem_to_chunk(p)) - OVERHEAD`。
  1. `mem_to_chunk(p)` 将用户指针偏移 `-OVERHEAD` 字节到内部 `Chunk` 头部。
  2. `chunk_size(c)` 读取 `c.csize` 并清除 `C_INUSE` 位得到原始 chunk 大小。
  3. 从原始 chunk 大小中减去 `OVERHEAD`，得到用户数据区实际可用字节数。

### 伪代码实现

```rust
extern "C" fn malloc_usable_size(p: *mut core::ffi::c_void) -> usize {
    if p.is_null() {
        return 0;
    }
    // SAFETY: p 是由本分配器返回的有效堆指针，前置条件保证了 Chunk 头部的有效性
    unsafe {
        let c: *const Chunk = mem_to_chunk(p);
        chunk_size(c) - OVERHEAD
    }
}
```

### 不变量

- 本函数为纯查询操作：不修改任何堆元数据，不持有锁，不分配或释放内存。
- 返回值始终满足 `返回值 >= 原始请求大小`（若原始请求大小已知）。
- 对于已释放的 chunk 调用本函数的结果不确定——`csize` 字段在 `free` 后可能被相邻空闲 chunk 合并逻辑覆写。

### 意图

提供一个 O(1) 的查询接口，使用户能够获知堆分配的实际可用空间上限。常用于：
- `realloc` 实现中快速判断是否需要实际搬迁数据（若原块空间足够则原地返回）。
- 用户态内存调试与统计。

### 算法复杂度

- 时间复杂度: **O(1)** — 一次空指针检查 + 一次指针偏移 + 一次内存读取 + 一次位掩码 + 一次减法。
- 空间复杂度: **O(1)** — 不分配额外内存。

### 错误处理

本函数**不设置 `errno`**。唯一的特殊情形是 `p.is_null()`，此时直接返回 0。
在 `no_std` 环境下，不依赖 `std::io::Error` 或 libc 的 `errno` 机制。

### 线程安全性

**线程安全**。本函数只读取传入指针所属 chunk 的元数据字段（`csize`），不修改任何共享状态，不持有任何锁。
在并发环境下与 `malloc` / `free` / `realloc` 同时调用是安全的——前提是传入的指针 `p` 在上述并发操作完成前保持有效
（即未被另一个线程 `free`）。

### Rust 类型映射与 ABI 兼容性说明

| C 类型 | Rust 类型 | 说明 |
|--------|----------|------|
| `size_t` | `usize` | 在所有 musl 支持的目标平台上，`usize` 与 `size_t` 具有相同宽度和对齐 |
| `void *` | `*mut core::ffi::c_void` | 使用 `core::ffi::c_void`，在 `no_std` 中可用 |
| `struct chunk *` | `*mut Chunk` / `*const Chunk` | 内部类型，仅在 `unsafe` 上下文中使用 |

**调用约定**: `extern "C"` 确保使用 C ABI 的调用约定，参数通过寄存器/栈按照目标平台的 C 调用约定传递。

---

## [RELY]

```
Predefined Types (来自 core, 所有 no_std 可用):
  core::ffi::c_void   — 对应 C 的 void 类型
  usize               — 对应 C 的 size_t

Predefined Internal Types / Constants / Functions (来自 oldmalloc 内部模块 malloc_impl.rs):
  struct Chunk  { psize: usize, csize: usize, next: *mut Chunk, prev: *mut Chunk }
                  — #[repr(C)] chunk 元数据结构体
  const OVERHEAD: usize = 2 * core::mem::size_of::<usize>()
                  — chunk 元数据开销常量
  const C_INUSE: usize = 1
                  — 使用中标志位常量
  unsafe fn chunk_size(c: *const Chunk) -> usize
                  — 读取 chunk 有效大小（清除 C_INUSE 位）
  unsafe fn mem_to_chunk(p: *mut c_void) -> *mut Chunk
                  — 用户指针到 chunk 指针的转换

Internal Cross-Module Dependencies (同一 crate 内, 由 Rust 编译器自然解析):
  fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void
                  — 定义于 rusl::malloc::oldmalloc::realloc
                  — 说明: realloc 内部调用 malloc_usable_size 以判断原地扩容，
                    形成循环引用。Rust 的 crate 内模块系统可自然处理此循环，
                    无需 C 的 __realloc_dep 链接器 hack。
```

---

## [GUARANTEE]

```
Exported Interface:
  extern "C" fn malloc_usable_size(p: *mut core::ffi::c_void) -> usize;
                  — 本模块保证对外提供的接口签名，ABI 兼容 C 版本。

对外导出的符号满足:
  - extern "C" 调用约定，参数类型和返回值类型与 C ABI 完全兼容
  - 参数顺序和语义与原 C 接口一致
  - 所有前置/后置条件与 C spec 保持一致
  - 线程安全，不修改全局状态
```

---

## 与 C 原版的设计差异

| 项目 | C 原版 (musl) | Rust 版 (rusl) |
|------|-------------|---------------|
| `malloc_usable_size` 签名 | `size_t malloc_usable_size(void *p)` | `extern "C" fn malloc_usable_size(p: *mut c_void) -> usize` |
| chunk 结构体 | `struct chunk { size_t psize, csize; struct chunk *next, *prev; };` | `#[repr(C)] pub(crate) struct Chunk { psize: usize, csize: usize, next: *mut Chunk, prev: *mut Chunk }` |
| `OVERHEAD` | `#define OVERHEAD (2*sizeof(size_t))` | `pub(crate) const OVERHEAD: usize = 2 * core::mem::size_of::<usize>()` |
| `CHUNK_SIZE(c)` | `#define CHUNK_SIZE(c) ((c)->csize & -2)` | `#[inline] unsafe fn chunk_size(c: *const Chunk) -> usize` |
| `MEM_TO_CHUNK(p)` | `#define MEM_TO_CHUNK(p) (struct chunk *)((char *)(p) - OVERHEAD)` | `#[inline] unsafe fn mem_to_chunk(p: *mut c_void) -> *mut Chunk` |
| `__realloc_dep` | `hidden void *(*const __realloc_dep)(void *, size_t) = realloc;` | **省略** — Rust 的 crate 编译模型自然处理模块间循环引用 |
| 内存安全 | 纯 C, 无安全保证, 依赖调用者正确性 | 内部使用 `unsafe` 封装指针运算，对外接口 unsafe 由调用者承担 |
| `#![no_std]` 兼容性 | N/A (C 无 std 概念) | 全程使用 `core::*`，不依赖 `std` 或 `libc` crate |

---

*本规约通过递归依赖追踪生成。内部类型/函数（`Chunk`, `OVERHEAD`, `chunk_size`, `mem_to_chunk`）在本文件中描述，定义于 `malloc_impl.rs`；跨文件依赖（`realloc`）标注引用来源。*