# aligned_alloc.c 规约 (oldmalloc)

> 源文件: `src/malloc/oldmalloc/aligned_alloc.c`
> 符号数量: 1 导出，0 内部 static
> 复杂度: Level 2（意图描述 + 前置/后置条件）

---

## 依赖图

```
aligned_alloc
  ├── malloc                    → src/malloc/oldmalloc/malloc.c   (外部模块, 见 malloc.c spec)
  ├── __bin_chunk               → src/malloc/oldmalloc/malloc.c   (外部模块, 见 malloc.c spec)
  ├── __malloc_replaced         → src/malloc/replaced.c           (外部模块, 见 replaced.c spec)
  ├── __aligned_alloc_replaced  → src/malloc/replaced.c           (外部模块, 见 replaced.c spec)
  ├── errno / EINVAL / ENOMEM   → 标准 C 库                      (跳过)
  ├── SIZE_MAX                  → <stdint.h>                      (跳过)
  ├── SIZE_ALIGN                → malloc_impl.h                   (本文件描述)
  ├── C_INUSE                   → malloc_impl.h                   (本文件描述)
  ├── IS_MMAPPED                → malloc_impl.h                   (本文件描述)
  ├── MEM_TO_CHUNK              → malloc_impl.h                   (本文件描述)
  ├── NEXT_CHUNK                → malloc_impl.h                   (本文件描述)
  └── struct chunk              → malloc_impl.h                   (本文件描述)
```

---

## 内部数据结构与宏

### `struct chunk` (内部类型)

```c
struct chunk {
    size_t psize, csize;
    struct chunk *next, *prev;
};
```

**[Visibility]: Internal** — musl oldmalloc 内部数据结构，定义于 `src/malloc/oldmalloc/malloc_impl.h`，不对外部用户暴露。

**语义**：
- `psize`：物理前驱 chunk 的大小（含 `C_INUSE` 标志位）。对于 mmap 分配的 chunk，此字段存储从 chunk 起始到 mmap 区域起始的偏移量（extra 字段）。
- `csize`：当前 chunk 的大小（含标志位）。最低位为 `C_INUSE` 标志。
- `next` / `prev`：当 chunk 空闲时，用于双链表 bin 管理；当 chunk 在用时，无意义。

### `SIZE_ALIGN` (宏)

```c
#define SIZE_ALIGN (4*sizeof(size_t))
```

**[Visibility]: Internal** — musl oldmalloc 内部宏。

**语义**：chunk 的最小对齐单位。在 32 位系统上为 16 字节，在 64 位系统上为 32 字节。所有由 `malloc` 返回的指针均已按此值对齐，因此 `align <= SIZE_ALIGN` 的 `aligned_alloc` 请求可直接委托给 `malloc`。

### `C_INUSE` (宏)

```c
#define C_INUSE  ((size_t)1)
```

**[Visibility]: Internal** — musl oldmalloc 内部宏。

**语义**：chunk 的占用标志位，位于 `csize` 的最低有效位。若该位为 1，表示当前 chunk 正在使用中或前一个 chunk 正在使用中（取决于读取上下文：`c->csize & C_INUSE` 表示当前 chunk 的使用状态，`c->psize & C_INUSE` 表示前一个 chunk 的使用状态）。

### `IS_MMAPPED` (宏)

```c
#define IS_MMAPPED(c) !((c)->csize & (C_INUSE))
```

**[Visibility]: Internal** — musl oldmalloc 内部宏。

**语义**：通过检查 `csize` 最低位是否为 0 来判断 chunk 是否由 `mmap` 直接分配。对于 mmap chunk，`C_INUSE` 位恒为 0（mmap chunk 永远不在常规 bin 中管理，无需该标志位）。

### `MEM_TO_CHUNK` (宏)

```c
#define MEM_TO_CHUNK(p) (struct chunk *)((char *)(p) - OVERHEAD)
```

其中 `OVERHEAD` = `2*sizeof(size_t)`。

**[Visibility]: Internal** — musl oldmalloc 内部宏。

**语义**：将用户可见的内存指针转换为对应的 chunk 结构体指针。chunk 头部位于用户内存之前 `OVERHEAD` 字节处。

### `NEXT_CHUNK` (宏)

```c
#define NEXT_CHUNK(c) ((struct chunk *)((char *)(c) + CHUNK_SIZE(c)))
```

其中 `CHUNK_SIZE(c)` = `((c)->csize & -2)`（剥离 `C_INUSE` 标志位）。

**[Visibility]: Internal** — musl oldmalloc 内部宏。

**语义**：根据当前 chunk 的大小，计算出物理后继 chunk 的地址。

---

## aligned_alloc (对外导出)

```c
void *aligned_alloc(size_t align, size_t len);
```

**[Visibility]: Public** — C11 标准函数，声明于 `<stdlib.h>`（§7.22.3.1）。

### 意图 (Intent)

按照给定的对齐要求从堆上分配内存。本实现是 musl oldmalloc（旧版 malloc 分配器）的对齐分配路径，核心策略为：先通过 `malloc` 分配比请求大小多 `align-1` 字节的原始内存，然后将返回指针向上对齐到 `align` 边界，最后将因对齐操作而产生的 leading fragment 作为一个新的空闲 chunk 归入 bin。

与 malloc-ng 版本不同，oldmalloc 的 `aligned_alloc` **不**强制要求 `len` 为 `align` 的整数倍（尽管 C11 标准有此项约束），musl 选择豁免此校验以保证兼容性。

### 前置条件 (Preconditions)

1. **对齐合法性校验**: `align` 必须是 2 的幂（即 `(align & -align) == align`），否则函数返回 NULL 并设置 `errno = EINVAL`。
2. **大小无溢出**: `len + align` 不得超出 `SIZE_MAX`（即 `len <= SIZE_MAX - align`），否则函数返回 NULL 并设置 `errno = ENOMEM`。
3. **替换一致性**: 若全局 `malloc` 已被用户替换（`__malloc_replaced != 0`）而 `aligned_alloc` 未被一同替换（`__aligned_alloc_replaced == 0`），则函数返回 NULL 并设置 `errno = ENOMEM`。这是因为 `aligned_alloc` 依赖内部分配器的 chunk 布局细节，在替换场景下无法安全实现。
4. **无锁要求**: 调用方无需持有任何锁。函数内部通过调用 `malloc` 和 `__bin_chunk`（后者内部持有 `split_merge_lock`）来处理并发。

### 后置条件 (Postconditions)

| 分支 | 条件 | 结果 |
|------|------|------|
| **EINVAL** | `align` 不是 2 的幂 | 返回 NULL，`errno = EINVAL`。无内存分配。 |
| **ENOMEM（溢出/替换）** | `len > SIZE_MAX - align` 或 `__malloc_replaced && !__aligned_alloc_replaced` | 返回 NULL，`errno = ENOMEM`。无内存分配。 |
| **小对齐委托** | `align <= SIZE_ALIGN` | 直接调用 `malloc(len)` 并返回其结果。因为 `malloc` 本身保证返回 `SIZE_ALIGN` 对齐，此举等价且更高效。 |
| **malloc 失败** | `malloc(len + align - 1)` 返回 NULL | 返回 NULL，`errno = ENOMEM`。 |
| **巧合对齐** | `malloc` 返回的原始指针恰好已满足 `align` 对齐 | 直接返回原始指针，不做 chunk 修改。无碎片产生。 |
| **mmap chunk 对齐** | 原始 chunk 为 mmap 块 (`IS_MMAPPED(c)` 为真) | 通过调整 `psize`（extra 偏移）和 `csize` 字段来记录对齐差值。返回对齐后的指针。无需分裂，无需归入 bin。 |
| **普通 chunk 分裂** | 原始 chunk 为普通堆块（非 mmap） | 将原始块分裂为两部分：(1) leading fragment（从原始 `mem` 到 `new` 之前）作为新的空闲 chunk 通过 `__bin_chunk(c)` 归入 bin；(2) aligned chunk（从 `new` 开始）作为本次分配的返回块。aligned chunk 的头尾（`n->psize` / `n->csize`）被设置为 `C_INUSE | (new-mem)` 大小，后继 chunk 的 `psize` 同步减量。 |

### 系统算法 (System Algorithm)

**Level 3** — 核心的内存分裂逻辑需要详细说明。

```
aligned_alloc(align, len):
  1. 校验 align 为 2 的幂，否则返回 EINVAL
  2. 校验 len + align 不溢出，且替换检测通过，否则返回 ENOMEM
  3. 若 align <= SIZE_ALIGN：直接 return malloc(len)
  4. 分配原始内存：mem = malloc(len + align - 1)
  5. 若 mem == NULL：返回 NULL (ENOMEM)
  6. 计算对齐地址：new = (mem + align - 1) & ~(align - 1)  // 向上对齐
  7. 若 new == mem：返回 mem（巧合对齐，无碎片）
  8. 若 IS_MMAPPED(MEM_TO_CHUNK(mem))：
     - n = MEM_TO_CHUNK(new)
     - n->psize = c->psize + (new - mem)   // 增大 extra 偏移
     - n->csize = c->csize - (new - mem)   // 减小有效大小
     - return new
  9. 普通 chunk 分裂：
     - c = MEM_TO_CHUNK(mem)
     - n = MEM_TO_CHUNK(new)
     - t = NEXT_CHUNK(c)                   // 原始 chunk 的后继 chunk
     - n->psize = c->csize = C_INUSE | (new - mem)  // aligned chunk 头
     - n->csize = t->psize -= (new - mem)           // aligned chunk 尾 + 后继 chunk 头
     - __bin_chunk(c)                      // 将 leading fragment 归入 free bin
     - return new
```

**关键设计细节**：

- **碎片回收**：对于非 mmap 的普通堆分配，leading fragment 通过 `__bin_chunk(c)` 释放回 bin 系统，`__bin_chunk` 内部会执行与前后相邻空闲 chunk 的合并（coalescing），确保碎片被高效回收。
- **大小字段语义**：分裂后，`n->psize` 和 `c->csize` 均被设为 `C_INUSE | (new - mem)`，其中 `C_INUSE` 标志位设为 1 表示该 leading fragment 占用中（一旦 `__bin_chunk` 将其释放为 free，该标志位将被清除），`(new - mem)` 为对齐跳过的字节数。
- **mmap chunk 特殊处理**：mmap 分配的 chunk 没有前后相邻的堆块，无法执行合并。因此对齐偏移被记录在 `psize`（extra 字段）中——`munmap` 时需要通过 `psize` 反推原始 `mmap` 基址。

### 不变量 (Invariants)

1. **chunk 大小一致性**：对于任何分裂操作，分裂后 `aligned chunk` 的 `csize` 与物理后继 chunk 的 `psize`（不含 `C_INUSE` 掩码部分）之和必须等于原始 chunk 的 `csize`。即：
   ```
   (n->csize & ~C_INUSE) + (c->csize & ~C_INUSE) == 原始 csize
   ```

2. **对齐后地址有效性**：`new` 指针必须满足 `(uintptr_t)new % align == 0` 且 `new >= mem` 且 `new - mem < align`。

3. **mmap psize 语义**：对于 mmap chunk，`psize` 字段存储的是从 chunk 结构体起始地址到 `mmap` 返回的原始基址的偏移量。该值在 `unmap_chunk` 中使用以正确计算 `munmap` 参数。

### 错误码

| errno 值 | 触发条件 |
|----------|----------|
| `EINVAL` | `align` 不是 2 的幂 |
| `ENOMEM` | `len > SIZE_MAX - align`（溢出）或 `__malloc_replaced && !__aligned_alloc_replaced`（替换不一致）或底层 `malloc` 返回 NULL |

### 边界情况

- **align = 0**：`(0 & -0) == 0` 为真，但 `align` 不为 2 的幂（零不是 2 的幂），因此 `(0 & -0) != 0` 为假，校验失败，返回 NULL + EINVAL。
- **align = 1**：`(1 & -1) == 1` 为真，`align <= SIZE_ALIGN` 成立，直接走 `malloc(len)` 路径。
- **len = 0**：`len > SIZE_MAX - align` 为假。进入 `malloc(len + align - 1)` 调用。若 `align <= SIZE_ALIGN`，走 `malloc(0)` 路径（行为见 `malloc` 规约）；否则走 `malloc(align - 1)` 路径。
- **超大对齐（如 align = SIZE_MAX/2 + 1）**：`len > SIZE_MAX - align` 校验会捕获，返回 ENOMEM。
- **小对齐正好等于 SIZE_ALIGN**：走 `malloc(len)` 路径，equivalent to ordinary malloc。

---

## 跨文件依赖说明

| 依赖符号 | 定义位置 | 性质 |
|----------|----------|------|
| `malloc` | `src/malloc/oldmalloc/malloc.c` | C 标准 Public API，跨模块依赖，详见 `malloc.c` spec |
| `__bin_chunk` | `src/malloc/oldmalloc/malloc.c:434` | `hidden void`，musl 内部函数，跨模块依赖，详见 `malloc.c` spec |
| `__malloc_replaced` | `src/malloc/replaced.c:3` | `hidden int`, 跨模块依赖，详见 `replaced.c` spec |
| `__aligned_alloc_replaced` | `src/malloc/replaced.c:4` | `hidden int`, 跨模块依赖，详见 `replaced.c` spec |
| `struct chunk` / 相关宏 | `src/malloc/oldmalloc/malloc_impl.h` | 内部类型/宏，本文件已描述 |

---

*本规约通过递归依赖追踪生成：`aligned_alloc` → `malloc` (跨文件) → `__bin_chunk` (跨文件) → `__malloc_replaced` / `__aligned_alloc_replaced` (跨文件，见 `replaced.c` spec) → `struct chunk` / `malloc_impl.h` 宏 (本文件描述)。*