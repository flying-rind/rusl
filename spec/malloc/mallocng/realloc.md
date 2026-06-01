# src/malloc/mallocng/realloc.c 规约

> **实现架构说明**：本文件是 musl mallocng 的 `realloc` 核心实现。通过 `glue.h` 中的 `#define realloc __libc_realloc`，本文件定义的 `realloc` 函数被重命名导出为内部符号 `__libc_realloc`。POSIX 公共入口 `src/malloc/realloc.c` 仅为薄封装，直接调用 `__libc_realloc`。本规约递归追踪 `__libc_realloc` 的全部内部依赖，按拓扑排序呈现。

---

## 依赖图

```
realloc (POSIX, src/malloc/realloc.c)
  └── __libc_realloc (即 mallocng realloc, src/malloc/mallocng/realloc.c)
        ├── malloc(n)  [= __libc_malloc_impl]  ── see mallocng/malloc.c spec
        ├── free(p)    [= __libc_free]         ── see mallocng/free.c spec
        ├── memcpy                             ── 外部 libc 函数
        ├── mremap / __mremap                  ── 外部 syscall 封装
        ├── size_overflows(n)                  ── 内部 inline (meta.h)
        ├── get_meta(p)                        ── 内部 inline (meta.h)
        ├── get_slot_index(p)                  ── 内部 inline (meta.h)
        ├── get_stride(g)                      ── 内部 inline (meta.h)
        ├── get_nominal_size(p, end)           ── 内部 inline (meta.h)
        ├── set_size(p, end, n)                ── 内部 inline (meta.h)
        ├── size_to_class(n)                   ── 内部 inline (meta.h)
        ├── assert                             ── 通过 glue.h 封装
        ├── struct meta / struct group          ── 内部类型 (meta.h)
        └── 常量: UNIT(=16), IB(=4), MMAP_THRESHOLD(=131052), MAP_FAILED, MREMAP_MAYMOVE
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

**[Visibility]: Internal** — musl mallocng 内部元数据结构，POSIX/C 标准未定义

**Intent**: 每个内存分配组 (group) 对应一个 `struct meta` 实例，记录该组的槽位可用性、空闲状态、大小类别、mmap 长度等元数据。`meta` 实例通过双向循环链表串联在 `ctx.active[sc]` 队列中。

**字段说明**:

| 字段 | 含义 |
|------|------|
| `prev`, `next` | 双向循环链表指针，用于将 meta 挂入 `ctx.active[sc]` 队列 |
| `mem` | 指向所属 `struct group` 的指针 |
| `avail_mask` | 位掩码，标记当前可用（已激活但尚未分配）的槽位 |
| `freed_mask` | 位掩码，标记已被释放的槽位 |
| `last_idx` | 该组中最后一个槽位的索引 (0-based)，组内槽位数为 `last_idx + 1` |
| `freeable` | 标记该组是否可以被整体回收释放（donate 产生的组标记为 0） |
| `sizeclass` | 大小类别编号 (0-47 为常规槽位类别，48+ 为大对象，63 表示单独 mmap 分配) |
| `maplen` | 若通过独立 mmap 分配，记录映射的页数（4096 字节为单位）；否则为 0 |

### `struct group` (内部类型)

```c
struct group {
    struct meta *meta;
    unsigned char active_idx:5;
    char pad[UNIT - sizeof(struct meta *) - 1];
    unsigned char storage[];
};
```

**[Visibility]: Internal** — musl mallocng 内部数据布局结构

**Intent**: 内存分配组的数据布局。`storage[]` 为柔性数组，包含 `last_idx+1` 个槽位，每个槽位跨度为 `stride` 字节。组头部（`struct group` 本身）占 `UNIT`(16) 字节，嵌入在槽位区域之前。

---

## 内部辅助函数 (来自 meta.h，为 realloc 直接依赖)

### `size_overflows` (内部函数)

```c
static inline int size_overflows(size_t n)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数，POSIX/C 标准未定义

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

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块的起始地址（用户可见指针）
- `p[-3]` 的低 5 位存储了槽位索引

**后置条件**:
- 返回 `p[-3] & 31`，即该指针所在组的槽位索引 (0-31)

---

### `get_meta` (内部函数)

```c
static inline struct meta *get_meta(const unsigned char *p)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向一个由 mallocng 分配的有效内存块起始地址
- `(uintptr_t)p` 为 16 字节对齐
- `p[-2]` 处存储了到组头部的偏移量（以 UNIT=16 为单位）
- 若 `p[-4]` 非零（表示使用了非零偏移 enframe），则 `p[-8]` 处存储 32 位扩展偏移量

**后置条件**:
- 通过双重间接寻址定位到 `struct meta`：
  1. 解析偏移量得到 `struct group *base`
  2. 通过 `base->meta` 得到 `struct meta *`
- 返回前执行完整的完整性断言检查：
  - `meta->mem == base`
  - `index <= meta->last_idx`
  - 该槽位既不在 `avail_mask` 也不在 `freed_mask` 中
  - `meta_area->check == ctx.secret`（防止元数据 corruption）
  - 对于 sizeclass < 48 的组，偏移量在对应类别允许的范围内
  - 对于 mmap 组 (maplen > 0)，偏移量不超过映射范围
  - sizeclass >= 48 时断言 `meta->sizeclass == 63`（始终为独立 mmap）

**Intent**: 从用户指针逆向定位到管理该内存块的 `struct meta`，是 mallocng 设计的核心——无需全局映射表即可在 O(1) 时间内查找到元数据。

---

### `get_nominal_size` (内部函数)

```c
static inline size_t get_nominal_size(const unsigned char *p, const unsigned char *end)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向该槽位的结束边界（已减去 IB 的哨兵空间）
- `p[-3]` 的高 3 位存储了保留大小编码

**后置条件**:
- 返回 `end - p - reserved`，即用户数据的实际可用大小 (`old_size`)
- `reserved` 的解析：
  - `reserved = p[-3] >> 5`，若 `reserved < 5`，直接使用
  - 若 `reserved == 5`，从 `end[-4]` 处读取 32 位扩展保留值（`assert(reserved >= 5)`）
- 断言检查：`reserved <= end-p`，`*(end-reserved) == 0`（哨兵字节），`*end == 0`（溢出检查字节）

**Intent**: 从编码的隐藏头部信息中解码出原始分配给用户的大小。在 realloc 中用于确定 `old_size`，以计算 memcpy 的拷贝量。

---

### `get_stride` (内部函数)

```c
static inline size_t get_stride(const struct meta *g)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `g` 指向有效的 `struct meta`

**后置条件**:
- Case 1 (mmap 单槽组): 若 `g->last_idx == 0 && g->maplen != 0`，返回 `g->maplen * 4096 - UNIT`
- Case 2 (常规组): 否则返回 `UNIT * size_classes[g->sizeclass]`

**Intent**: 返回该组中单个槽位的总跨度（stride），即相邻槽位起始地址之间的字节数。用于计算 `start`（槽位起始）和 `end`（槽位可用末尾）。

---

### `size_to_class` (内部函数)

```c
static inline int size_to_class(size_t n)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `n` 为用户请求的分配大小（字节）

**后置条件**:
- 返回对应的大小类别编号 (0-47)，用于索引 `size_classes[]` 和 `ctx.active[]`
- 算法：
  1. `n = (n + IB - 1) >> 4` — 将字节数向上取整为 16 字节单元数
  2. 若 `n < 10`，直接返回 `n`（类别 0-9 为精确值）
  3. 否则 `n++`，使用 `a_clz_32` 计算前导零数量，结合固定查找表 `size_classes[]` 确定类别

**Intent**: 将用户请求大小映射到 mallocng 的 48 个大小类别之一，实现对数级分类。在 realloc 中用于判断原地扩/缩容是否可行（新大小类别与原类别必须相邻或更大）。

---

### `set_size` (内部函数)

```c
static inline void set_size(unsigned char *p, unsigned char *end, size_t n)
```

**[Visibility]: Internal** — musl mallocng 内部辅助函数

**前置条件**:
- `p` 指向用户内存块起始地址
- `end` 指向槽位边界（`p + stride - IB`）
- `n` 为新的用户可用大小，满足 `n <= end - p`（即不超过槽位容量）

**后置条件**:
- 将新的大小 `n` 编码到隐藏头部：
  - `reserved = end - p - n`（尾部保留字节数）
  - 若 `reserved > 0`，在 `end[-reserved]` 处写入哨兵字节 0
  - 若 `reserved >= 5`，在 `end[-4]` 处写入 32 位扩展保留值，在 `end[-5]` 处写入哨兵字节 0，然后 `reserved = 5`
  - 将 `p[-3]` 更新为 `(p[-3] & 31) | (reserved << 5)`（保留低 5 位的 slot index，高 3 位编码 reserved）

**Intent**: 将新分配大小编码到内存块的隐藏头部。realloc 在原地缩容/扩容和 mremap 成功后使用此函数更新记录的大小。

---

## 内部实现函数

### `__libc_realloc` (内部符号)

```c
void *realloc(void *p, size_t n);
// 通过 glue.h: #define realloc __libc_realloc 导出为 __libc_realloc
```

**[Visibility]: Internal** — musl 内部实现函数。通过 `glue.h` 宏重命名为 `__libc_realloc`。用户程序应通过 `<stdlib.h>` 调用 POSIX 标准函数 `realloc`（由 `src/malloc/realloc.c` 提供薄封装）。

**Intent**: musl mallocng 的 realloc 核心实现。采用多级策略，按优先级递减尝试：原地大小调整（最优，零拷贝） → mremap 重映射（mmap 大块场景） → malloc+memcpy+free（通用回退路径）。尽量减少数据拷贝和系统调用。

---

#### 前置条件

- 若 `p != NULL`，`p` 必须是先前由 `malloc()`、`calloc()`、`realloc()` 或兼容分配函数返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放
- 若 `p == NULL`，行为等同于 `malloc(n)`
- 无特定锁持有要求（内部通过 `malloc`/`free` 自行管理锁）

#### 后置条件

**Case 1: `p == NULL` (等效于 malloc)**

- 直接调用 `malloc(n)` 分配新内存
- 返回分配得到的指针，若分配失败则返回 `NULL`，`errno = ENOMEM`

**Case 2: `n` 导致溢出 (`size_overflows(n)` 为真)**

- 返回 `NULL`，设置 `errno = ENOMEM`
- 原内存块 `p` 保持有效且未被释放

**Case 3: 原地缩容/扩容 (最优路径，零拷贝)**

- **触发条件** (三个条件同时满足):
  1. `n <= avail_size` — 新大小不超过槽位可用空间
  2. `n < MMAP_THRESHOLD` (131052 字节) — 不触发大块阈值
  3. `size_to_class(n) + 1 >= g->sizeclass` — 新大小类别与原类别相同、相邻或更大（即大小退化不跨越类别边界）

- **计算过程**:
  - `g = get_meta(p)` — 定位元数据
  - `idx = get_slot_index(p)` — 获取槽位索引
  - `stride = get_stride(g)` — 获取槽位跨度
  - `start = g->mem->storage + stride * idx` — 槽位起始地址
  - `end = start + stride - IB` — 槽位可用末尾（减去 IB 哨兵空间）
  - `avail_size = end - (unsigned char *)p` — 从用户指针到槽位末尾的可用字节数

- **动作**: 调用 `set_size(p, end, n)` 就地更新记录的大小
- **返回**: 原指针 `p`（内存地址不变）
- **数据完整性**: 原有数据在 `min(旧大小, n)` 范围内保持不变

**Case 4: mremap 重映射 (mmap 大块优化路径)**

- **触发条件** (两个条件同时满足):
  1. `g->sizeclass >= 48` — 原块为大对象（独立 mmap 分配）
  2. `n >= MMAP_THRESHOLD` (131052 字节) — 新大小也达到大块阈值

- **前置断言**: `g->sizeclass == 63`（确保是独立 mmap 分配，非子分配组）

- **计算过程**:
  - `base = (unsigned char *)p - start` — 用户数据在 mmap 区域内的偏移量
  - `needed = (n + base + UNIT + IB + 4095) & -4096` — 向上取整到页对齐的新映射大小
    - `n`：用户请求大小
    - `base`：从映射起始到用户数据的偏移（包含组头部和 enframe 偏移）
    - `UNIT`：组头部大小 (16 字节)
    - `IB`：尾部哨兵空间 (4 字节)
    - `+ 4095` 后 `& -4096`：向上取整到页边界

- **子情况 4a: 新大小恰好等于原大小**
  - `g->maplen * 4096UL == needed` — 无需重新映射
  - 直接复用 `new = g->mem`，跳过 mremap 系统调用

- **子情况 4b: 需要 mremap**
  - 调用 `mremap(g->mem, g->maplen*4096UL, needed, MREMAP_MAYMOVE)`
  - `MREMAP_MAYMOVE` 标志允许内核移动映射到新地址
  - `new = mremap(...)` 获取新映射地址

- **成功处理**:
  - 若 `new != MAP_FAILED`:
    - 更新元数据：`g->mem = new`，`g->maplen = needed / 4096`
    - 重新计算用户指针和边界：
      - `p = g->mem->storage + base`
      - `end = g->mem->storage + (needed - UNIT) - IB`
    - 写入尾部哨兵：`*end = 0`
    - 调用 `set_size(p, end, n)` 更新大小记录
    - 返回更新后的 `p`
  - 若 `new == MAP_FAILED`，**不回退** — mremap 失败时原映射保持不变，继续执行 Case 5

- **失败处理**:
  - 若 `mremap` 返回 `MAP_FAILED`，代码**不会立即返回 NULL**，而是继续执行 Case 5 的 malloc+memcpy+free 回退路径

**Case 5: malloc+memcpy+free (通用回退路径)**

- **触发条件**: Case 3 和 Case 4 的条件均不满足，或 Case 4 的 mremap 失败
- **动作**:
  1. `new = malloc(n)` — 分配新内存块
  2. 若 `!new`，返回 `NULL`，`errno = ENOMEM`，原块 `p` 保持有效
  3. `memcpy(new, p, n < old_size ? n : old_size)` — 将旧数据拷贝到新块（取新旧大小的较小值）
  4. `free(p)` — 释放旧内存块
  5. 返回 `new`
- **数据完整性**: 原有数据被完整拷贝到新地址（上限为 `min(old_size, n)`），超出部分未初始化

---

#### 不变量

- **Inv 1 (数据安全)**: 在 Case 2 失败和 Case 5 malloc 失败时，原内存块 `p` 始终保持有效且内容不变。调用者必须在失败时继续持有 `p` 并在后续显式 `free(p)`
- **Inv 2 (原地调整安全性)**: Case 3 的原地缩容/扩容保证了 `n <= end - p`，即新大小不超过槽位物理容量，不会越界写入
- **Inv 3 (sizeclass 单调性)**: Case 3 中的条件 `size_to_class(n) + 1 >= g->sizeclass` 确保新大小类别不低于原类别太多。例如原块在类别 10（分配 32-42 单元），缩容到仅需 5 单元时，新类别 5 远小于原类别 10，不满足条件，会走 Case 5 分配更小类别的新块——这避免了在大槽位中浪费空间
- **Inv 4 (mmap 大小一致性)**: 在 Case 4 成功后，`g->maplen` 总是等于 `needed / 4096`，即映射的页数精确对应计算出的页对齐大小
- **Inv 5 (哨兵字节)**: 每次成功调用 `set_size` 后，`end[-reserved]`（或 `end[-5]` 当 reserved >= 5 时）和 `*end` 处均有哨兵字节 0，用于 free 时的完整性验证

---

#### 系统算法 (System Algorithm)

**Level 1: 元数据定位阶段**

```
g = get_meta(p)           // 从用户指针逆向定位 struct meta
idx = get_slot_index(p)   // 提取槽位索引 (0-31)
stride = get_stride(g)    // 计算槽位跨度
start = g->mem->storage + stride * idx  // 槽位起始地址
end = start + stride - IB               // 槽位末尾（减去 4 字节哨兵空间）
old_size = get_nominal_size(p, end)     // 解码原始分配大小
avail_size = end - (unsigned char *)p   // 当前可用空间
```

**Level 2: 三路策略选择**

```
if (n <= avail_size && n < MMAP_THRESHOLD && size_to_class(n)+1 >= g->sizeclass)
    → PATH A: 原地更新 (set_size) → return p

if (g->sizeclass >= 48 && n >= MMAP_THRESHOLD)
    // assert(g->sizeclass == 63)
    → PATH B: mremap 重映射
    if (mremap 成功)
        → 更新 g->mem, g->maplen, set_size → return p

→ PATH C: malloc + memcpy + free → return new
```

**PATH C 的正确性**: 即使在 PATH B (mremap) 失败后也走 PATH C，因为 mremap 失败时原映射保持不变，内核不会改变 `p` 的有效性。此时代码直接进入 `new = malloc(n)`，若成功则 memcpy+free(p)，若失败则返回 NULL 且 `p` 仍然有效。

---

## 对外导出函数

### `realloc` (对外导出)

```c
void *realloc(void *p, size_t n);
```

**[Visibility]: Public** — POSIX.1-2001 / ISO C89 标准函数，`<stdlib.h>` 声明

**意图**: 更改 `p` 指向的内存块大小为 `n` 字节。musl 中通过两层架构实现：
- `src/malloc/realloc.c` — 公共入口，直接转发到 `__libc_realloc`
- `src/malloc/mallocng/realloc.c` — 核心实现（即本文件）

**前置条件**:
- 若 `p != NULL`，`p` 必须是先前由 `malloc()`、`calloc()`、`realloc()`、`aligned_alloc()` 或 `posix_memalign()` 返回的有效指针，且尚未被 `free()` 或 `realloc()` 释放
- 若 `p == NULL`，函数等价于 `malloc(n)`
- 若 `n == 0` 且 `p != NULL`，musl 的行为等价于 `free(p)` 并返回 `NULL`（或一个可被 `free` 的唯一指针，具体由底层实现决定）

**后置条件**:
- **成功**: 返回指向新分配内存块的指针
  - 若原地调整或 mremap 成功，返回的指针可能等于原 `p`（数据在 `min(旧大小, n)` 范围内保留）
  - 若需要移动（PATH C），返回新指针，旧块已被释放，新块包含来自旧块的 `min(旧大小, n)` 字节数据
  - 若 `n > 旧大小`，超出部分的**内容未初始化**
  - 新指针保持适合任何类型的对齐
- **失败**: 返回 `NULL`，`errno = ENOMEM`，原内存块 `p` **保持不变**且仍然有效，调用者必须后续显式 `free(p)`

**线程安全性**: 通过内部 `malloc`/`free` 的锁机制保证线程安全。`__libc_realloc` 自身不直接获取锁，而是依赖被调用的 `malloc`/`free` 内部加锁。

**信号安全性**: 不是 async-signal-safe。持有锁期间被信号中断可能导致死锁。

---

## 常量定义

| 常量 | 定义位置 | 值 | 在 realloc 中的用途 |
|------|---------|-----|-------------------|
| `UNIT` | meta.h | 16 | 基本分配单元大小（字节），组头部大小，偏移量计算单位 |
| `IB` | meta.h | 4 | 槽位末尾保留的 in-band 元数据/哨兵字节数 |
| `MMAP_THRESHOLD` | meta.h | 131052 | 超过此大小的分配使用独立 mmap 而非槽位分配。用于判断原地调整（Case 3 要求 `n < MMAP_THRESHOLD`）和 mremap 路径（Case 4 要求 `n >= MMAP_THRESHOLD`） |
| `MREMAP_MAYMOVE` | `<sys/mman.h>` | — | mremap 标志，允许内核移动映射到新地址 |
| `MAP_FAILED` | `<sys/mman.h>` | `(void *)-1` | mmap/mremap 失败时的返回值 |

---

## 跨文件依赖说明

| 依赖项 | 来源 | 类型 | 说明 |
|--------|------|------|------|
| `__libc_malloc_impl` (`malloc`) | `src/malloc/mallocng/malloc.c` (via `glue.h`) | 内部实现 | Case 1 和 Case 5 中分配新内存。详见 mallocng/malloc.c spec |
| `__libc_free` (`free`) | `src/malloc/mallocng/free.c` (via `glue.h`) | 内部实现 | Case 5 中释放旧内存。详见 mallocng/free.c spec |
| `memcpy` | `<string.h>` | 外部 libc 函数 | Case 5 中拷贝数据 |
| `mremap` / `__mremap` | `<sys/mman.h>` (via `glue.h`) | 外部 syscall 封装 | Case 4 中重映射 mmap 区域 |
| `assert` | `glue.h` (via `<assert.h>` or `a_crash`) | 内部宏 | Case 4 中断言 `g->sizeclass == 63` |
| `size_classes[]` | `src/malloc/mallocng/malloc.c` (via `meta.h`) | 内部全局数组 | 48 个元素的大小类别表 |
| `ctx` | `src/malloc/mallocng/malloc.c` (via `meta.h`) | 内部全局变量 | 全局分配器上下文（含 secret、active[]、usage_by_class[] 等） |
| 内联辅助函数 (`get_meta`, `get_stride` 等) | `src/malloc/mallocng/meta.h` | 内部 inline 函数 | 已在本文档详述 |

---

## 安全考虑

1. **整数溢出防护**: 函数入口立即检查 `size_overflows(n)`，防止 `n` 过大导致后续计算（如 `needed = (n + base + UNIT + IB + 4095) & -4096`）溢出

2. **元数据 corruption 检测**: `get_meta(p)` 内含多层断言：
   - `meta_area->check == ctx.secret` — 防止伪造/损坏的 meta area
   - 偏移量范围检查 — 确保指针确实指向有效槽位
   - `avail_mask`/`freed_mask` 检查 — 确保槽位当前处于"已分配"状态

3. **mremap 失败安全**: 当 `mremap` 返回 `MAP_FAILED` 时，内核保证原映射保持不变。代码不会在此处返回 NULL，而是优雅回退到 malloc+memcpy+free (PATH C)

4. **失败不回退陷阱**: 若 PATH C 中 `malloc(n)` 成功但后续 `memcpy` 或 `free` 触发段错误（如 `p` 已损坏），则新块 `new` 会泄漏。这是 realloc 语义的内在限制——一旦进入"分配新块"阶段，无法再安全回退到原块