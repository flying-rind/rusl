# aligned_alloc Rust 接口

> 源 C spec: `src/malloc/oldmalloc/spec/aligned_alloc.md`
> 复杂度: Level 2（意图描述 + 前置/后置条件）
> 对外导出符号: 1（`aligned_alloc`），内部符号全部重新设计

---

## 依赖图

```
aligned_alloc
  ├── malloc                    → malloc 模块         (外部 Public API, 见 malloc.c spec)
  ├── __bin_chunk               → malloc 模块         (外部 Internal, 见 malloc.c spec)
  ├── __malloc_replaced         → replaced 模块       (外部 Internal, 见 replaced.c spec)
  ├── __aligned_alloc_replaced  → replaced 模块       (外部 Internal, 见 replaced.c spec)
  ├── Chunk / SIZE_ALIGN / OVERHEAD / C_INUSE → malloc_impl 模块 (内部类型/常量)
  ├── is_mmapped / chunk_size / mem_to_chunk → malloc_impl 模块 (内部辅助函数)
  ├── next_chunk / chunk_to_mem → malloc_impl 模块    (内部辅助函数)
  └── errno / EINVAL / ENOMEM   → errno 模块          (平台相关, 跳过)
```

---

## 内部数据结构与辅助类型（Rust 重新设计）

以下所有内部类型、常量、函数均为 `pub(crate)` 可见性，定义于 `malloc_impl` 模块，不对外部用户暴露。

### `Chunk` 结构体

```rust
/// 堆块元数据结构。采用边界标记（boundary tag）设计。
/// 必须使用 `#[repr(C)]` 以确保与 C 分配器代码的 ABI 布局兼容，
/// 因为 `__bin_chunk` 等内部函数仍为 C ABI。
#[repr(C)]
pub(crate) struct Chunk {
    pub(crate) psize: usize,  // 物理前驱 chunk 大小（含 C_INUSE 标志位）
    pub(crate) csize: usize,  // 当前 chunk 大小（含标志位）
    pub(crate) next: *mut Chunk,  // 空闲链表后继（仅空闲时有效）
    pub(crate) prev: *mut Chunk,  // 空闲链表前驱（仅空闲时有效）
}
```

**[Visibility]: Internal** — 仅在 `malloc_impl` 模块内定义，`pub(crate)` 导出给老版 malloc 子模块使用。

**字段语义**：
- `psize`：物理前驱 chunk 的大小（含 `C_INUSE` 标志位于 bit 0）。对于 mmap 分配的 chunk，该字段存储从 chunk 起始到 mmap 区域起始的偏移量（extra 字段）。
- `csize`：当前 chunk 的大小（含标志位于 bit 0）。`csize & 1` 表示 `C_INUSE` 标志。
- `next` / `prev`：当 chunk 空闲时用于双链表 bin 管理；当 chunk 在用时，该内存区域属于用户数据区。

### 对齐与大小常量

```rust
/// chunk 的最小对齐单位。
/// 64 位系统上为 32 字节，32 位系统上为 16 字节。
pub(crate) const SIZE_ALIGN: usize = 4 * core::mem::size_of::<usize>();

/// 每个 chunk 的元数据开销 = psize + csize 两个 usize 字段。
pub(crate) const OVERHEAD: usize = 2 * core::mem::size_of::<usize>();

/// chunk 占用标志位。位于 csize/psize 的最低有效位 (bit 0)。
/// 所有 chunk 大小均为 SIZE_ALIGN 的整数倍，最低位天然为 0，可安全复用。
pub(crate) const C_INUSE: usize = 1;
```

**[Visibility]: Internal** — 仅在 `malloc_impl` 模块内定义。

### Chunk 导航与转换辅助函数

```rust
/// 返回 chunk 的实际大小，剥离 C_INUSE 标志位。
/// `csize & !1` 等价于 C 宏 `(c)->csize & -2`。
#[inline]
pub(crate) const fn chunk_size(c: &Chunk) -> usize {
    c.csize & !C_INUSE
}

/// 返回前一个物理 chunk 的实际大小，剥离 C_INUSE 标志位。
#[inline]
pub(crate) const fn chunk_psize(c: &Chunk) -> usize {
    c.psize & !C_INUSE
}

/// 判断正在使用中的 chunk 是否由 mmap 直接分配。
/// 对于 mmap chunk，C_INUSE 位恒为 0。
#[inline]
pub(crate) fn is_mmapped(c: &Chunk) -> bool {
    c.csize & C_INUSE == 0
}

/// 将用户可见的内存指针转换为对应的 Chunk 元数据指针。
/// 用户内存位于 chunk 头部之后 OVERHEAD 字节处。
#[inline]
pub(crate) fn mem_to_chunk(p: *mut core::ffi::c_void) -> *mut Chunk {
    unsafe { (p as *mut u8).sub(OVERHEAD) as *mut Chunk }
}

/// 将 Chunk 元数据指针转换为用户可见的内存指针。
#[inline]
pub(crate) fn chunk_to_mem(c: *const Chunk) -> *mut core::ffi::c_void {
    unsafe { (c as *const u8).add(OVERHEAD) as *mut core::ffi::c_void }
}

/// 根据当前 chunk 的大小，计算出物理后继 chunk 的地址。
#[inline]
pub(crate) fn next_chunk(c: *mut Chunk) -> *mut Chunk {
    unsafe { (c as *mut u8).add(chunk_size(&*c)) as *mut Chunk }
}

/// 计算物理前驱 chunk 的地址。
#[inline]
pub(crate) fn prev_chunk(c: *mut Chunk) -> *mut Chunk {
    unsafe { (c as *mut u8).sub(chunk_psize(&*c)) as *mut Chunk }
}
```

**[Visibility]: Internal** — 仅在 `malloc_impl` 模块内定义。使用 Rust `#[inline]` 标记确保零成本抽象，编译后与原 C 宏等效。

**设计说明**：
- 原 C 宏 `CHUNK_SIZE(c)`、`IS_MMAPPED(c)` 等在 Rust 中重新设计为函数（而非宏），利用 Rust 的类型系统和 `unsafe` 块明确标注不安全操作边界。
- `mem_to_chunk` 和 `chunk_to_mem` 内部包含指针运算，标记为 `unsafe` 调用，将安全责任显式化。
- 使用 `const fn`（如 `chunk_size`）允许在编译期常量求值，优化性能。

---

## Rust 接口

### aligned_alloc (对外导出)

```rust
// [Visibility]: Public — C11 标准函数，声明于 <stdlib.h> (§7.22.3.1)
#[no_mangle]
pub unsafe extern "C" fn aligned_alloc(align: usize, len: usize) -> *mut core::ffi::c_void;
```

### 意图 (Intent)

按照给定的对齐要求从堆上分配内存。本实现是 musl oldmalloc（旧版 malloc 分配器）的对齐分配路径，核心策略为：先通过 `malloc` 分配比请求大小多 `align - 1` 字节的原始内存，然后将返回指针向上对齐到 `align` 边界，最后将因对齐操作而产生的 leading fragment 作为一个新的空闲 chunk 归入 bin。

与 malloc-ng 版本不同，oldmalloc 的 `aligned_alloc` **不**强制要求 `len` 为 `align` 的整数倍（尽管 C11 标准有此项约束），musl 选择豁免此校验以保证兼容性。

### 前置条件 (Preconditions)

1. **对齐合法性校验**: `align` 必须是 2 的幂（即 `align.count_ones() == 1`），否则函数返回 `null` 并设置 `errno = EINVAL`。
2. **大小无溢出**: `len + align` 不得超出 `usize::MAX`（即 `len <= usize::MAX - align`），否则函数返回 `null` 并设置 `errno = ENOMEM`。
3. **替换一致性**: 若全局 `malloc` 已被用户替换（`__malloc_replaced != 0`）而 `aligned_alloc` 未被一同替换（`__aligned_alloc_replaced == 0`），则函数返回 `null` 并设置 `errno = ENOMEM`。这是因为 `aligned_alloc` 依赖内部分配器的 chunk 布局细节，在替换场景下无法安全实现。
4. **无锁要求**: 调用方无需持有任何锁。函数内部通过调用 `malloc` 和 `__bin_chunk`（后者内部持有 `split_merge_lock`）来处理并发。

### 后置条件 (Postconditions)

| 分支 | 条件 | 结果 |
|------|------|------|
| **EINVAL** | `align` 不是 2 的幂 | 返回 `null`，`errno = EINVAL`。无内存分配。 |
| **ENOMEM（溢出/替换）** | `len > usize::MAX - align` 或 `__malloc_replaced && !__aligned_alloc_replaced` | 返回 `null`，`errno = ENOMEM`。无内存分配。 |
| **小对齐委托** | `align <= SIZE_ALIGN` | 直接调用 `malloc(len)` 并返回其结果。因为 `malloc` 本身保证返回 `SIZE_ALIGN` 对齐，此举等价且更高效。 |
| **malloc 失败** | `malloc(len + align - 1)` 返回 `null` | 返回 `null`，`errno = ENOMEM`。 |
| **巧合对齐** | `malloc` 返回的原始指针恰好已满足 `align` 对齐 | 直接返回原始指针，不做 chunk 修改。无碎片产生。 |
| **mmap chunk 对齐** | 原始 chunk 为 mmap 块（`is_mmapped(c)` 为 `true`） | 通过调整 `psize`（extra 偏移）和 `csize` 字段来记录对齐差值。返回对齐后的指针。无需分裂，无需归入 bin。 |
| **普通 chunk 分裂** | 原始 chunk 为普通堆块（非 mmap） | 将原始块分裂为两部分：(1) leading fragment（从原始 `mem` 到 `new` 之前）作为新的空闲 chunk 通过 `__bin_chunk(c)` 归入 bin；(2) aligned chunk（从 `new` 开始）作为本次分配的返回块。aligned chunk 的头尾（`n.psize` / `n.csize`）被设置为 `C_INUSE | (new - mem)` 大小，后继 chunk 的 `psize` 同步调整。 |

### 系统算法 (System Algorithm)

**Level 3** — 核心的内存分裂逻辑需要详细说明。

```
aligned_alloc(align, len):
  1. 校验 align 为 2 的幂（(align & (align - 1)) == 0），否则返回 EINVAL
  2. 校验 len + align 不溢出，且替换检测通过，否则返回 ENOMEM
  3. 若 align <= SIZE_ALIGN：直接 return malloc(len)
  4. 分配原始内存：let mem = malloc(len + align - 1);
  5. 若 mem.is_null()：返回 null (ENOMEM)
  6. 计算对齐地址：let new = ((mem as usize + align - 1) & !(align - 1)) as *mut c_void;
  7. 若 new == mem：返回 mem（巧合对齐，无碎片）
  8. 若 is_mmapped(&*mem_to_chunk(mem))：
     - let c = mem_to_chunk(mem);
     - let n = mem_to_chunk(new);
     - (*n).psize = (*c).psize + (new as usize - mem as usize);  // 增大 extra 偏移
     - (*n).csize = (*c).csize - (new as usize - mem as usize);  // 减小有效大小
     - return new
  9. 普通 chunk 分裂：
     - let c = mem_to_chunk(mem);
     - let n = mem_to_chunk(new);
     - let t = next_chunk(c);                        // 原始 chunk 的后继 chunk
     - (*n).psize = (*c).csize = C_INUSE | (new as usize - mem as usize);
     - (*n).csize = (*t).psize -= (new as usize - mem as usize);
     - __bin_chunk(c);                               // 将 leading fragment 归入 free bin
     - return new
```

**Rust 实现关键设计细节**：

- **不安全性集中管理**：所有原始指针操作（来自 `malloc` 返回、`mem_to_chunk` 转换、chunk 字段读取/写入）均在明确的 `unsafe` 块内进行。Rust 的类型系统确保这些不安全操作不会泄漏到安全代码中。
- **碎片回收**：对于非 mmap 的普通堆分配，leading fragment 通过 `__bin_chunk(c)` 释放回 bin 系统，`__bin_chunk` 内部会执行与前后相邻空闲 chunk 的合并（coalescing），确保碎片被高效回收。
- **大小字段语义**：分裂后，`n.psize` 和 `c.csize` 均被设为 `C_INUSE | (new - mem)`，其中 `C_INUSE` 标志位设为 1 表示该 leading fragment 占用中（一旦 `__bin_chunk` 将其释放为 free，该标志位将被清除），`(new - mem)` 为对齐跳过的字节数。
- **mmap chunk 特殊处理**：mmap 分配的 chunk 没有前后相邻的堆块，无法执行合并。因此对齐偏移被记录在 `psize`（extra 字段）中——`munmap` 时需要通过 `psize` 反推原始 `mmap` 基址。

### 不变量 (Invariants)

1. **chunk 大小一致性**：对于任何分裂操作，分裂后 aligned chunk 的 `csize` 与 leading fragment 的 `csize`（均不含 `C_INUSE` 掩码部分）之和必须等于原始 chunk 的 `csize`。即：
   ```
   chunk_size(n) + chunk_size(c) == 原始 csize
   ```

2. **对齐后地址有效性**：`new` 指针必须满足 `(new as usize) % align == 0` 且 `new >= mem` 且 `new as usize - mem as usize < align`。

3. **mmap psize 语义**：对于 mmap chunk，`psize` 字段存储的是从 chunk 结构体起始地址到 `mmap` 返回的原始基址的偏移量。该值在 `unmap_chunk` 中使用以正确计算 `munmap` 参数。

### 错误码

| errno 值 | 触发条件 |
|----------|----------|
| `EINVAL` | `align` 不是 2 的幂 |
| `ENOMEM` | `len > usize::MAX - align`（溢出）或 `__malloc_replaced && !__aligned_alloc_replaced`（替换不一致）或底层 `malloc` 返回 `null` |

### 边界情况

- **align = 0**：`0 & !0 == 0`，但 0 不是 2 的幂（`0.count_ones() != 1`），校验失败，返回 null + EINVAL。
- **align = 1**：`1.count_ones() == 1` 为真，`align <= SIZE_ALIGN` 成立，直接走 `malloc(len)` 路径。
- **len = 0**：`len > usize::MAX - align` 为假。进入 `malloc(len + align - 1)` 调用。若 `align <= SIZE_ALIGN`，走 `malloc(0)` 路径（行为见 `malloc` 规约）；否则走 `malloc(align - 1)` 路径。
- **超大对齐（如 align = usize::MAX/2 + 1）**：`len > usize::MAX - align` 校验会捕获，返回 ENOMEM。
- **小对齐正好等于 SIZE_ALIGN**：走 `malloc(len)` 路径，等效于普通 malloc。

---

## 跨模块依赖说明

| 依赖符号 | 定义位置 | 性质 |
|----------|----------|------|
| `malloc` | `malloc_old` 模块 (`src/malloc/oldmalloc/malloc.rs`) | C 标准 Public API，跨模块依赖 |
| `__bin_chunk` | `malloc_old` 模块 (`src/malloc/oldmalloc/malloc.rs`) | musl Internal（`hidden` 可见性），跨模块依赖 |
| `__malloc_replaced` | `replaced` 模块 (`src/malloc/replaced.rs`) | musl Internal（`hidden` 可见性），跨模块依赖 |
| `__aligned_alloc_replaced` | `replaced` 模块 (`src/malloc/replaced.rs`) | musl Internal（`hidden` 可见性），跨模块依赖 |
| `Chunk` / 相关常量和辅助函数 | `malloc_impl` 模块 | 内部类型/函数，本文件已描述 |

---

## [RELY]

```
Cross-module Functions (外部 Public API):
  // 定义于 malloc_old 模块
  pub unsafe extern "C" fn malloc(size: usize) -> *mut core::ffi::c_void;
    // C 标准 Public API，ABI 兼容

Cross-module Functions (外部 Internal, hidden 可见性):
  // 定义于 malloc_old 模块
  fn __bin_chunk(self_: *mut Chunk);
    // musl 内部函数，hidden 可见性，不对外导出
    // 将 chunk 归还到空闲链表（bin）中，并与前后物理相邻的空闲 chunk 尝试合并

Cross-module Variables (外部 Internal, hidden 可见性):
  // 定义于 replaced 模块
  static mut __malloc_replaced: core::ffi::c_int;
    // musl 内部状态变量，hidden 可见性
    // 0 = malloc 未被外部插替, 非0 = 已被替换
  static mut __aligned_alloc_replaced: core::ffi::c_int;
    // musl 内部状态变量，hidden 可见性
    // 0 = aligned_alloc 未被外部插替, 非0 = 已被替换

Internal Types and Helpers (定义于 malloc_impl 模块, pub(crate)):
  #[repr(C)]
  struct Chunk {
      psize: usize, csize: usize,
      next: *mut Chunk, prev: *mut Chunk,
  }
  const SIZE_ALIGN: usize = 4 * core::mem::size_of::<usize>();
  const OVERHEAD: usize = 2 * core::mem::size_of::<usize>();
  const C_INUSE: usize = 1;
  fn chunk_size(c: &Chunk) -> usize;      // c.csize & !C_INUSE
  fn chunk_psize(c: &Chunk) -> usize;     // c.psize & !C_INUSE
  fn is_mmapped(c: &Chunk) -> bool;       // c.csize & C_INUSE == 0
  fn mem_to_chunk(p: *mut c_void) -> *mut Chunk;
  fn chunk_to_mem(c: *const Chunk) -> *mut c_void;
  fn next_chunk(c: *mut Chunk) -> *mut Chunk;
  fn prev_chunk(c: *mut Chunk) -> *mut Chunk;

Platform Dependencies (errno 模块):
  // 定义于平台相关 errno 模块
  fn set_errno(code: core::ffi::c_int);
  const EINVAL: core::ffi::c_int;
  const ENOMEM: core::ffi::c_int;
```

---

## [GUARANTEE]

```
Exported Interface:
  #[no_mangle]
  pub unsafe extern "C" fn aligned_alloc(align: usize, len: usize) -> *mut core::ffi::c_void;
    // 本模块保证对外提供的接口签名
    // C11 标准 aligned_alloc 函数（7.22.3.1节）
    // ABI 兼容：参数/返回值布局与 C 版本完全一致
    // 满足所有原 C spec 的前置/后置条件和不变量约束

  // 无其他对外导出符号
```

---

*本 Rust spec 通过递归依赖追踪生成：`aligned_alloc` -> `malloc` (跨模块) -> `__bin_chunk` (跨模块) -> `__malloc_replaced` / `__aligned_alloc_replaced` (跨模块，见 `replaced` 模块) -> `Chunk` / `malloc_impl` 辅助类型和函数 (本文件描述, 使用 Rust `const fn` / `#[inline] fn` 重新设计为类型安全和零成本抽象)。*