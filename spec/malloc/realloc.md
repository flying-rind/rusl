# src/malloc/realloc.c 规约

## 概述

`realloc.c` 是 musl libc 中 POSIX `realloc` 函数的入口文件。该文件本身仅是一个薄封装层，实际的内存管理逻辑由 musl 内部的新一代 malloc 实现 "mallocng" 提供。`realloc` 通过 glue 层宏重命名机制间接调用 `mallocng/realloc.c` 中定义的 `__libc_realloc`。

本规约递归追踪 `realloc` 的全部内部依赖，按拓扑排序呈现。

---

## 依赖图

```
realloc (Public, stdlib.h)
  └── __libc_realloc (Internal, mallocng/realloc.c)
        ├── malloc(n) [= __libc_malloc_impl] ── see mallocng/malloc.c spec
        ├── free(p)  [= __libc_free]        ── see mallocng/free.c spec
        ├── memcpy                           ── 外部 libc 函数
        ├── mremap                           ── 外部 syscall 封装
        ├── size_overflows(n)                ── 内部 inline (meta.h)
        ├── get_meta(p)                      ── 内部 inline (meta.h)
        ├── get_slot_index(p)                ── 内部 inline (meta.h)
        ├── get_stride(g)                    ── 内部 inline (meta.h)
        ├── get_nominal_size(p, end)         ── 内部 inline (meta.h)
        ├── set_size(p, end, n)             ── 内部 inline (meta.h)
        ├── size_to_class(n)                 ── 内部 inline (meta.h)
        ├── struct meta                      ── 内部类型 (meta.h)
        └── 常量: UNIT, IB, MMAP_THRESHOLD  ── (meta.h)
```

---

## 内部类型定义

### `struct meta` (内部类型)

```c
struct meta {
    struct meta *prev, *next;
    struct group *mem;
    volatile int avail_mask, freed_mask;
    uintptr_t last_idx:5;
    uintptr_t freeable:1;
    uintptr_t sizeclass:6;
    uintptr_t maplen:8*sizeof(uintptr_t)-12;
};
```

[Visibility]: Internal — musl mallocng 内部元数据结构，POSIX/C 标准未定义

**Intent**: 每个内存分配组 (group) 对应一个 `struct meta` 实例，记录该组的槽位可用性、空闲状态、大小类别、mmap 长度等元数据。`meta` 实例通过双向链表串联在 `ctx.active[sc]` 队列中。

**字段说明**:
| 字段 | 含义 |
|------|------|
| `prev`, `next` | 双向链表指针，用于将 meta 挂入 active 队列 |
| `mem` | 指向所属 `struct group` 的指针 |
| `avail_mask` | 位掩码，标记当前可用（已激活但未分配）的槽位 |
| `freed_mask` | 位掩码，标记已被释放的槽位 |
| `last_idx` | 该组中最后一个槽位的索引 (0-based)，组内槽位数为 `last_idx+1` |
| `freeable` | 标记该组是否可以被整体回收释放 |
| `sizeclass` | 大小类别编号 (0-47 为常规类别, 48+ 为特殊类别, 63 表示单独 mmap) |
| `maplen` | 若通过独立 mmap 分配，记录映射的页数 (4096 字节为单位)；否则为 0 |

---

## 内部辅助函数 (meta.h inline functions)

### `size_overflows` (内部函数)

```c
static inline int size_overflows(size_t n)
```

[Visibility]: Internal — musl mallocng 内部辅助函数，POSIX/C 标准未定义

**前置条件**:
- 无特定要求

**后置条件**:
- Case 1 (溢出): 若 `n >= SIZE_MAX/2 - 4096`，设置 `errno = ENOMEM`，返回 1
- Case 2 (正常): 否则返回 0，`errno` 不变

**Intent**: 在分配前检查请求大小是否会导致后续计算溢出（如加上 IB+UNIT 后溢出 SIZE_MAX）。

---

### `get_slot_index` (内部函数)

```c
static inline int get_slot_index(const unsigned char *p)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块的起始地址（用户可见指针）
- `p[-3]` 的低 5 位存储了槽位索引

**后置条件**:
- 返回 `p[-3] & 31`，即该指针所在组的槽位索引 (0-31)

**Intent**: 从用户指针的隐藏头部字节中提取槽位索引。

---

### `get_meta` (内部函数)

```c
static inline struct meta *get_meta(const unsigned char *p)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块起始地址
- `(uintptr_t)p` 为 16 字节对齐 (`assert(!((uintptr_t)p & 15))`)
- `p[-2]` 处存储了到组头部的偏移量 (以 UNIT=16 为单位)
- 若 `p[-4]` 非零（表示使用了非零偏移 enframe），则 `p[-8]` 处存储 32 位扩展偏移量

**后置条件**:
- 通过双重间接寻址定位到 `struct meta`：
  1. 解析偏移量得到 `struct group *base`
  2. 通过 `base->meta` 得到 `struct meta *`
- 返回前进行一系列断言检查，确保数据结构完整性：
  - `meta->mem == base`
  - `index <= meta->last_idx`
  - 该槽位既不在 `avail_mask` 也不在 `freed_mask` 中
  - 对于 sizeclass < 48 的组，偏移量在对应类别允许的范围内
  - 对于 mmap 组 (maplen > 0)，偏移量不超过映射范围

**Intent**: 从用户指针逆向定位到管理该内存块的 `struct meta`，这是 musl mallocng 设计的关键——无需额外的映射表即可在 O(1) 时间内查找到元数据。

---

### `get_nominal_size` (内部函数)

```c
static inline size_t get_nominal_size(const unsigned char *p, const unsigned char *end)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向该槽位的结束边界（已减去 IB）
- `p[-3]` 的高 3 位存储了保留大小编码

**后置条件**:
- 返回 `end - p - reserved`，即用户数据的实际可用大小
- 其中 `reserved` 的解析：
  - `reserved = p[-3] >> 5`，若 `reserved < 5`，直接使用
  - 若 `reserved == 5`，从 `end[-4]` 处读取 32 位扩展保留值
- 断言检查：`reserved <= end-p`，尾部哨兵字节为 0

**Intent**: 从编码的头部信息中解码出分配给用户的真实大小（不含尾部保留空间）。

---

### `get_stride` (内部函数)

```c
static inline size_t get_stride(const struct meta *g)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `g` 指向有效的 `struct meta`

**后置条件**:
- Case 1 (mmap 单槽组): 若 `g->last_idx == 0 && g->maplen != 0`，返回 `g->maplen * 4096 - UNIT`
- Case 2 (常规组): 否则返回 `UNIT * size_classes[g->sizeclass]`

**Intent**: 返回该组中单个槽位的总跨度（stride），即相邻槽位起始地址之间的字节数。对于 mmap 组，整个映射空间为一个槽位。

---

### `size_to_class` (内部函数)

```c
static inline int size_to_class(size_t n)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `n` 为用户请求的分配大小（字节）

**后置条件**:
- 返回对应的大小类别编号 (0-47)，用于索引 `size_classes[]` 和 `ctx.active[]`
- 算法：
  1. `n = (n + IB - 1) >> 4` — 将字节数向上取整为 16 字节单元数
  2. 若 `n < 10`，直接返回 `n`（类别 0-9 为精确值）
  3. 否则 `n++`，使用 `a_clz_32` 计算前导零数量，结合固定查找表确定类别

**Intent**: 将用户请求大小映射到 mallocng 的 48 个大小类别之一，实现对数级分类，确保每个类别内的分配大小差异可控。

---

### `set_size` (内部函数)

```c
static inline void set_size(unsigned char *p, unsigned char *end, size_t n)
```

[Visibility]: Internal — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向槽位边界（`p + stride - IB`）
- `n` 为新的用户可用大小，满足 `n <= end - p`（即不超过槽位容量）

**后置条件**:
- 将新的大小 `n` 编码到隐藏头部：
  - `reserved = end - p - n`（尾部保留字节数）
  - 若 `reserved > 0`，在 `end[-reserved]` 处写入哨兵 0
  - 若 `reserved >= 5`，在 `end[-4]` 处写入 32 位扩展值，在 `end[-5]` 处写入哨兵 0
  - 将 `p[-3]` 更新为 `(p[-3] & 31) | (reserved << 5)`

**Intent**: 将新的分配大小编码到内存块的隐藏头部。`realloc` 在原地扩容/缩容时使用此函数更新记录的大小。

---

## 内部实现函数

### `__libc_realloc` (内部函数)

```c
void *realloc(void *p, size_t n)  // 通过 glue.h: #define realloc __libc_realloc
```

[Visibility]: Internal — musl 内部实现函数，通过 glue.h 宏重命名为 `__libc_realloc`。用户程序应调用 POSIX 标准函数 `realloc`

**Intent**: musl mallocng 的 realloc 核心实现。优先尝试在原地调整大小，避免数据拷贝。按策略优先级递减：原地缩容/扩容 → mremap 重映射 → malloc+memcpy+free。

**前置条件**:
- 若 `p != NULL`，`p` 必须是先前由 `malloc`/`calloc`/`realloc` 返回的有效指针
- 若 `p == NULL`，行为等同于 `malloc(n)`

**后置条件**:

**Case 1: `p == NULL` (等效于 malloc)**
- 调用 `__libc_malloc_impl(n)` 分配新内存，返回其指针
- 若分配失败，返回 `NULL`

**Case 2: `n` 导致溢出 (`size_overflows(n)` 为真)**
- 返回 `NULL`，`errno = ENOMEM`，原内存块 `p` 保持有效

**Case 3: 原地缩容/扩容 (最优路径)**
- 条件: `n <= avail_size`（请求大小不超过槽位可用空间），且 `n < MMAP_THRESHOLD`（131052 字节），且 `size_to_class(n) + 1 >= g->sizeclass`（新大小类别与原类别相邻或更大）
- 动作: 调用 `set_size(p, end, n)` 就地更新记录的大小，返回原指针 `p`
- 该路径避免任何数据拷贝和系统调用

**Case 4: mremap 重映射 (mmap 大块路径)**
- 条件: `g->sizeclass >= 48 && n >= MMAP_THRESHOLD`（原块和大块阈值）
  - 且 `g->sizeclass == 63`（断言确保为独立 mmap 分配）
- 动作:
  - 计算所需的新映射大小 `needed = (n + base + UNIT + IB + 4095) & -4096`（页对齐）
  - 若 `g->maplen * 4096 == needed`（新大小恰好等于原大小），直接复用 `g->mem`
  - 否则调用 `mremap(g->mem, g->maplen*4096, needed, MREMAP_MAYMOVE)`
  - 若 `mremap` 成功：
    - 更新 `g->mem` 和 `g->maplen`
    - 更新 `end` 指向新映射的末尾
    - 写入尾部哨兵，调用 `set_size(p, end, n)` 更新大小记录
    - 返回重映射后的指针
  - 若 `mremap` 返回 `MAP_FAILED`，回退到 Case 5

**Case 5: malloc+memcpy+free (通用回退路径)**
- 调用 `__libc_malloc_impl(n)` 分配新块
- 若分配失败，返回 `NULL`，原块 `p` 保持有效
- 调用 `memcpy(new, p, min(n, old_size))` 将旧数据拷贝到新块
- 调用 `__libc_free(p)` 释放旧块
- 返回新块指针 `new`

**不变量**:
- 在 Case 3/4 成功时，原有数据在 `min(旧大小, n)` 范围内保持不变
- 在 Case 5 成功时，原有数据被拷贝到新地址
- 任何失败路径（Case 2 失败、Case 5 malloc 失败）下，原内存块 `p` 保持不变且仍可用

---

## 对外导出函数

### `realloc` (对外导出)

```c
void *realloc(void *p, size_t n);
```

[Visibility]: Public — POSIX.1-2001 / C89 标准函数，`<stdlib.h>` 声明

**意图**: 更改 `p` 指向的内存块大小为 `n` 字节。实现委托给内部 `__libc_realloc`。

**前置条件**:
- 若 `p != NULL`，`p` 必须是先前由 `malloc()`、`calloc()`、`realloc()` 返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放
- 若 `p == NULL`，函数等价于 `malloc(n)`
- 若 `n == 0` 且 `p != NULL`，行为是实现定义的（musl 将其视为等价于 `free(p)` 并返回 `NULL`，但不在本文件中处理——该逻辑在 `__libc_realloc` 内部处理）

**后置条件**:
- Case 1 (成功): 返回指向新分配内存块的指针
  - 若 `p == NULL`，行为同 `malloc(n)`
  - 若 `n <= 旧大小` 且原地缩容，可能返回原指针（内容截断为 `n`）
  - 若需要移动，新块内容保留 `min(旧大小, n)` 字节来自旧块的数据，返回新指针
  - 若 `n > 旧大小`，超出旧块大小的部分未初始化
- Case 2 (失败): 返回 `NULL`，`errno = ENOMEM`，原内存块 `p` 保持不变且仍然有效
- Case 3 (`n == 0` 且 `p != NULL`): 行为由 `__libc_realloc` 决定（musl 中可能等价于 `free(p)` 返回 `NULL`）

**实现架构**:
```
src/malloc/realloc.c          -- 公共入口，委托给 __libc_realloc
src/malloc/mallocng/glue.h    -- 宏重命名: #define realloc __libc_realloc
src/malloc/mallocng/realloc.c -- 核心实现逻辑
src/malloc/mallocng/meta.h    -- 元数据结构定义与内联辅助函数
```

**线程安全性**: 通过 `__libc_realloc` 间接持有 `__malloc_lock`（在 malloc/free 内部处理锁），本文件不直接涉及锁操作。

---

## 常量定义

| 常量 | 定义位置 | 值 | 含义 |
|------|---------|-----|------|
| `UNIT` | meta.h | 16 | 基本分配单元大小（字节），所有对齐的基础 |
| `IB` | meta.h | 4 | 槽位末尾保留的 in-band 元数据字节数 |
| `MMAP_THRESHOLD` | meta.h | 131052 | 超过此大小的分配使用独立 mmap 而非槽位分配 |
| `size_classes[]` | mallocng/malloc.c | (48 个元素) | 每个大小类别对应的最大分配单元数 |

---

## 跨文件依赖说明

| 依赖项 | 来源 | 类型 |
|--------|------|------|
| `__libc_realloc` | `src/malloc/mallocng/realloc.c` (via `glue.h`) | 内部实现 |
| `__libc_malloc_impl` (`malloc`) | `src/malloc/mallocng/malloc.c` (via `glue.h`) | 内部实现，见 mallocng/malloc.c spec |
| `__libc_free` (`free`) | `src/malloc/mallocng/free.c` (via `glue.h`) | 内部实现，见 mallocng/free.c spec |
| `memcpy` | `<string.h>` | 外部 libc 函数 |
| `mremap` | `<sys/mman.h>` (via `glue.h` → `__mremap`) | 外部 syscall 封装 |
| `struct meta`, 内联辅助函数 | `src/malloc/mallocng/meta.h` | 内部类型/函数，已在上方描述 |