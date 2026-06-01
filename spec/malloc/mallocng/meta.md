# meta.h 规约

> 本文件是 musl mallocng (malloc new generation) 分配器的核心头文件，定义了分配器所需的全部数据结构、常量和内联辅助函数。

---

## 依赖图

```
meta.h
├── 外部依赖 (来自标准头文件 / 其他内部模块)
│   ├── <stdint.h>       → uint16_t, uint32_t, uint64_t, uint8_t, uintptr_t, size_t
│   ├── <errno.h>        → ENOMEM, errno
│   ├── <limits.h>       → SIZE_MAX
│   ├── glue.h           → size_classes[], ctx 的宏命名空间 (see glue.h spec)
│   │   ├── atomic.h     → a_cas(), a_clz_32() (see atomic.h spec)
│   │   ├── <pthread.h>  → pthread locking (跳过, POSIX)
│   │   ├── <sys/mman.h> → mmap/madvise (跳过, 系统调用)
│   │   └── ...
│   └── assert.h (通过 glue.h 的 USE_REAL_ASSERT 或 a_crash)
│
├── 数据结构 (本文件定义, Insider Internal)
│   ├── struct group
│   ├── struct meta
│   ├── struct meta_area
│   └── struct malloc_context
│
├── 导出全局符号 (hidden visibility, 跨 .c 共享, Internal)
│   ├── size_classes[]   → 定义于 malloc.c
│   ├── ctx              → 定义于 malloc.c
│   ├── alloc_meta()     → 定义于 donate.c (推测)
│   └── is_allzero()     → 定义于 malloc.c (推测)
│
└── 内联辅助函数 (static inline, Internal)
    ├── queue / dequeue / dequeue_head  → 元数据链表操作
    ├── free_meta                       → [依赖 queue]
    ├── activate_group                  → [依赖 a_cas]
    ├── get_slot_index / get_meta       → 分配指针 → 元数据逆向解析
    ├── get_nominal_size / set_size     → 分配块大小编解码
    ├── get_stride                      → 组内槽位大小计算
    ├── enframe                         → [依赖 get_stride, set_size]
    ├── size_to_class / size_overflows  → [依赖 size_classes[], a_clz_32]
    └── step_seq / record_seq / account_bounce / decay_bounces / is_bouncing → 反碎片化序列号系统
```

---

## 常量定义

### MMAP_THRESHOLD

```c
#define MMAP_THRESHOLD 131052
```

[Visibility]: Internal -- musl mallocng 内部阈值，POSIX/C 标准未定义

**意图**: 当分配请求大小超过此阈值 (约 128KB) 时，分配器绕过 slab 机制，直接使用 `mmap` 独立分配，并独立 `munmap` 释放。

---

### UNIT

```c
#define UNIT 16
```

[Visibility]: Internal -- musl mallocng 内部常量

**意图**: 最小分配对齐粒度。所有分配以 16 字节为步进单位。该值与 x86-64 ABI 对齐要求一致，同时作为 `struct group` 的 header 偏移量。

---

### IB

```c
#define IB 4
```

[Visibility]: Internal -- musl mallocng 内部常量

**意图**: In-band header size，即每个槽位底部的"带内"元数据开销（4 字节，位于用户可用区域末尾之后）。每个 allocation slot 的实际存储空间为 `stride` 字节，其中 `IB` 字节用作越界检查标记。

---

### PGSZ

```c
#ifdef PAGESIZE
#define PGSZ PAGESIZE
#else
#define PGSZ ctx.pagesize
#endif
```

[Visibility]: Internal -- musl mallocng 内部宏

**意图**: 页大小。若编译时可确定（`PAGESIZE` 已定义），则使用编译常量；否则在运行时从 `ctx.pagesize` 读取（由动态链接器或 `sysconf` 在初始化阶段填入）。

---

## 数据结构

### struct group

```c
struct group {
    struct meta *meta;
    unsigned char active_idx:5;
    char pad[UNIT - sizeof(struct meta *) - 1];
    unsigned char storage[];
};
```

[Visibility]: Internal -- musl mallocng 内部分配组结构，POSIX/C 标准未定义

**意图**: 一组相同大小类别内存槽位的容器。是 slab 分配的基本单位。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `meta` | `struct meta *` | 指向本组元数据的反向指针，用于从 `storage[]` 中的指针快速定位元数据 |
| `active_idx` | `unsigned char:5` | 当前活动掩码的最高位编号 (0..31)，指示空闲槽位 `freed_mask` 中哪一位已被该组"认领" |
| `pad` | `char[N]` | 填充至 `UNIT` 字节对齐 |
| `storage` | `unsigned char[]` | 柔性数组，实际存储区域，按 `stride` 步进划分槽位 |

**不变量**:
- `group->meta->mem == group` -- 元数据与组的双向绑定必须一致
- 整个 `group` 起始地址按页对齐（由 `mmap` 保证）
- `storage` 区域中的每个槽位前 `IB` 字节为 in-band header，后 `IB` 字节为保留校验区

---

### struct meta

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

[Visibility]: Internal -- musl mallocng 内部元数据结构，POSIX/C 标准未定义

**意图**: 描述一个 `group` 的内存使用状态，同时充当链表节点存在于多种队列中（按 `sizeclass` 对应的 active 链表、free_meta 链表等）。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `prev` / `next` | `struct meta *` | 双向循环链表指针，该 meta 在 active 链表或 free_meta 链表中的位置 |
| `mem` | `struct group *` | 指向所描述的 `struct group` |
| `avail_mask` | `volatile int` | 可用槽位位掩码，位 i 为 1 表示槽位 i 空闲可分配 |
| `freed_mask` | `volatile int` | 释放槽位位掩码，位 i 为 1 表示槽位 i 已被释放但尚未被 `activate_group` 认领 |
| `last_idx` | `uintptr_t:5` | 本组内的最大槽位索引（槽位数 - 1，最大 31） |
| `freeable` | `uintptr_t:1` | 标记该组是否可以整体释放（通过 `madvise` 或 `munmap`） |
| `sizeclass` | `uintptr_t:6` | 大小类别索引 (0..47 + 63 表示 mmap 大对象) |
| `maplen` | `uintptr_t:N` | mmap 映射长度（以页为单位），对于非 mmap 组，该字段总为 0 |

**不变量**:
- 当路径经过 `get_meta()` 校验时，必须满足 `meta->mem == base` 且 `index <= meta->last_idx`
- `avail_mask` 和 `freed_mask` 不相交（同一槽位不能同时处于可用和已释放状态）
- 若 `meta->prev == NULL && meta->next == NULL`，则该 meta 不在任何队列中
- 位域打包后 `sizeof(struct meta) <= 32`（通常为 4 个指针 + 2 个 int = 32 字节）

---

### struct meta_area

```c
struct meta_area {
    uint64_t check;
    struct meta_area *next;
    int nslots;
    struct meta slots[];
};
```

[Visibility]: Internal -- musl mallocng 内部结构，POSIX/C 标准未定义

**意图**: 按页对齐的内存区域，用于批量分配 `struct meta`。每个 meta_area 包含一个校验值、链表指针和若干 meta 槽位。该区域本身通过 `mmap` 分配，起始地址 4KB 对齐。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `check` | `uint64_t` | 安全校验值，应等于 `ctx.secret`，用于防止伪造的指针攻击 |
| `next` | `struct meta_area *` | 链表指针，链接所有 meta_area 实例 |
| `nslots` | `int` | 槽位数量 |
| `slots` | `struct meta[]` | 柔性数组，实际的 meta 槽位 |

**不变量**:
- `area->check == ctx.secret` -- 每次通过地址反查必须验证
- `(uintptr_t)area & 4095 == 0` -- 页对齐
- 有效 meta 的地址满足 `(uintptr_t)meta & -4096 == (uintptr_t)area`

---

### struct malloc_context

```c
struct malloc_context {
    uint64_t secret;
#ifndef PAGESIZE
    size_t pagesize;
#endif
    int init_done;
    unsigned mmap_counter;
    struct meta *free_meta_head;
    struct meta *avail_meta;
    size_t avail_meta_count, avail_meta_area_count, meta_alloc_shift;
    struct meta_area *meta_area_head, *meta_area_tail;
    unsigned char *avail_meta_areas;
    struct meta *active[48];
    size_t usage_by_class[48];
    uint8_t unmap_seq[32], bounces[32];
    uint8_t seq;
    uintptr_t brk;
};
```

[Visibility]: Internal -- musl mallocng 全局分配上下文，POSIX/C 标准未定义

**意图**: 线程安全的全局分配器状态。整个 musl mallocng 分配器共享唯一一个 `struct malloc_context` 实例 `ctx`。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `secret` | `uint64_t` | 随机密钥，用于 meta_area 校验和地址混淆，在 `malloc` 首次调用时初始化 |
| `pagesize` | `size_t` | 运行时页大小（仅当编译时未定义 PAGESIZE 时存在） |
| `init_done` | `int` | 初始化完成标志，0 表示未初始化 |
| `mmap_counter` | `unsigned` | mmap 调用计数器，用于触发周期性元数据回收 |
| `free_meta_head` | `struct meta *` | 空闲 meta 双向循环链表头 |
| `avail_meta` | `struct meta *` | 可用的 meta 区域起始指针 |
| `avail_meta_count` | `size_t` | 可用 meta 计数 |
| `avail_meta_area_count` | `size_t` | 可用 meta_area 计数 |
| `meta_alloc_shift` | `size_t` | meta 区域分配的指数增长因子 |
| `meta_area_head` | `struct meta_area *` | meta_area 链表头 |
| `meta_area_tail` | `struct meta_area *` | meta_area 链表尾 |
| `avail_meta_areas` | `unsigned char *` | 可用 meta_area 位图 |
| `active[48]` | `struct meta *[48]` | 每个 sizeclass 的活跃 meta 双向循环链表头（48 个 size class + 0 用于 non-sizeclass） |
| `usage_by_class[48]` | `size_t[48]` | 每个 sizeclass 的累计使用量 |
| `unmap_seq[32]` | `uint8_t[32]` | 每个 size class (7-38) 最后一次 unmap 操作序列号 |
| `bounces[32]` | `uint8_t[32]` | 每个 size class 的"弹跳"计数（频繁 map/unmap 的惩罚因子） |
| `seq` | `uint8_t` | 全局操作序列计数器 (1-255)，每次分配/释放步进，用于检测 unmap 抖动 |
| `brk` | `uintptr_t` | 当前 brk 值（程序堆末端），用于扩展初始堆区域 |

**不变量**:
- `active[i]` 要么为 NULL（空链表），要么指向一个有效的双向循环链表头（`head->prev->next == head`）
- `free_meta_head` 要么为 NULL，要么指向有效双向循环链表头
- 全局 `ctx` 实例的访问必须在持有 `__malloc_lock` 的情况下进行（多线程安全）

---

## 内联函数

### queue

```c
static inline void queue(struct meta **phead, struct meta *m);
```

[Visibility]: Internal -- musl mallocng 内部链表操作函数，POSIX/C 标准未定义

**意图**: 将 meta 节点插入双向循环链表尾部（效果上插入到头节点的前面）。

**前置条件**:
- `phead` 非 NULL，指向链表头指针
- `m` 非 NULL，且 `m->next == NULL && m->prev == NULL`（节点当前不在任何链表中）
- `*phead` 要么为 NULL，要么指向一个有效的循环链表

**后置条件** (Case 1: 链表原为空 `*phead == NULL`):
- `m->prev == m->next == m`（自环）
- `*phead == m`

**后置条件** (Case 2: 链表非空 `*phead != NULL`):
- `m` 被插入到 `*phead` 之前（循环链表的尾部）
- `m->prev` 指向旧的尾部节点，`m->next` 指向 `*phead`
- 循环链表完整性保持

**系统算法**: O(1) 循环链表尾部插入。利用 C 指针共享的经典循环链表插入模式 `m->next->prev = m->prev->next = m`。

---

### dequeue

```c
static inline void dequeue(struct meta **phead, struct meta *m);
```

[Visibility]: Internal -- musl mallocng 内部链表操作函数，POSIX/C 标准未定义

**意图**: 从双向循环链表中移除 meta 节点。

**前置条件**:
- `phead` 非 NULL
- `m` 非 NULL，且 `m` 必须在 `*phead` 指向的链表中

**后置条件** (Case 1: 链表只剩一个节点 `m->next == m`):
- `*phead = NULL`
- `m->prev = m->next = NULL`

**后置条件** (Case 2: 链表有多个节点 `m->next != m`):
- `m` 从链表中移除，前后节点正确重链
- 若 `*phead == m`，则 `*phead` 更新为 `m->next`
- `m->prev = m->next = NULL`

**系统算法**: O(1) 循环链表删除。先处理前后重链，再处理头指针更新。

---

### dequeue_head

```c
static inline struct meta *dequeue_head(struct meta **phead);
```

[Visibility]: Internal -- musl mallocng 内部链表操作函数，POSIX/C 标准未定义

**意图**: 从双向循环链表中取出并返回头节点。

**前置条件**:
- `phead` 非 NULL

**后置条件** (Case 1: 链表为空 `*phead == NULL`):
- 返回 `NULL`

**后置条件** (Case 2: 链表非空):
- 返回原 `*phead`，该节点已从链表中移除，`prev`/`next` 清零

**系统算法**: O(1)，委托给 `dequeue()`。

---

### free_meta

```c
static inline void free_meta(struct meta *m);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 将使用完毕的 meta 结构体清零并回收到全局 `ctx.free_meta_head` 空闲链表中。

**前置条件**:
- `m` 非 NULL，指向一个不再使用的 `struct meta`
- 调用者持有 malloc 锁

**后置条件**:
- `m` 所有字段被清零 (`*m = (struct meta){0}`)
- `m` 被加入 `ctx.free_meta_head` 链表

**依赖**: `queue()` (本文件)

---

### activate_group

```c
static inline uint32_t activate_group(struct meta *m);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 通过原子 CAS 操作将 `freed_mask` 中 `active_idx` 范围内的已释放槽位转移到 `avail_mask` 中，使其变为可分配状态。这是连接 "free" 操作与 "malloc" 操作的关键桥梁。

**前置条件**:
- `m` 非 NULL
- `m->avail_mask == 0`（组当前无可分配槽位，才会触发 activate）
- 调用者持有 malloc 锁（至少 rdlock）

**后置条件**:
- `m->avail_mask` 包含原 `freed_mask` 中在 `active_idx` 位范围内的所有位
- `m->freed_mask` 中被认领的位已通过 CAS 原子清除
- 返回值为 `m->avail_mask` 的新值

**系统算法**: 使用 `a_cas` 原子 CAS 循环。计算公式 `act = (2u<<m->mem->active_idx)-1` 构造掩码，一次性原子地从 `freed_mask` 中取出低位释放槽位。

---

### get_slot_index

```c
static inline int get_slot_index(const unsigned char *p);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 从分配指针的 in-band header 中提取槽位索引。

**前置条件**:
- `p` 指向一个已分配块的起始地址

**后置条件**:
- 返回 `p[-3] & 31`，即 header 字节的低 5 位（0-31 的槽位索引）

**不变量**: 每个分配指针 `p` 满足 `p[-3] & 31 == index`，其中 `index` 为槽位在 group 中的编号。

---

### get_meta

```c
static inline struct meta *get_meta(const unsigned char *p);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 从任意分配指针逆向推导对应的 `struct meta`。这是 musl 设计中最核心的安全校验函数，通过多重断言确保指针合法性，防止 double-free、伪造指针等攻击。

**前置条件**:
- `p` 非 NULL，`(uintptr_t)p` 为 16 字节对齐
- `p` 指向一个由 mallocng 分配的合法内存块

**后置条件** (Case 1: 所有断言通过):
- 返回该块所属 group 的 `struct meta *`

**后置条件** (Case 2: 任一断言失败):
- `a_crash()` -- 进程立即终止（防内存损坏传播）

**校验链** (按顺序):
1. `assert(!((uintptr_t)p & 15))` -- 地址 16 字节对齐
2. 读取 `p[-2]` 作为 16 位偏移量，`get_slot_index(p)` 获取槽位索引
3. 若 `p[-4]` 非零，表明使用了非零起始偏移，则偏移量实际存储于 `p[-8]`，且 `assert(offset > 0xffff)`
4. 计算 group 基址 `base = p - UNIT*offset - UNIT`
5. 通过 `base->meta` 获取元数据指针
6. `assert(meta->mem == base)` -- 双向绑定验证
7. `assert(index <= meta->last_idx)` -- 索引不越界
8. `assert(!(meta->avail_mask & (1u<<index)))` -- 槽位不空闲（已分配状态）
9. `assert(!(meta->freed_mask & (1u<<index)))` -- 槽位未被释放
10. 计算 `meta_area` 指针（页对齐向下取整）
11. `assert(area->check == ctx.secret)` -- 密钥验证防伪造
12. 对于 `sizeclass < 48`，验证偏移量与 sizeclass 的一致性
13. 对于 `sizeclass == 63`（mmap 大对象），确认 `meta->sizeclass == 63`
14. 若 `meta->maplen` 非零，验证偏移量不超过页映射范围

**系统算法**: 多重层叠校验。使用 `assert` 实现零开销（release 构建下全部移除），同时利用 in-band header 中的自描述信息实现 O(1) 反向查找。

---

### get_nominal_size

```c
static inline size_t get_nominal_size(const unsigned char *p, const unsigned char *end);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 从分配块的 header 中恢复用户原始请求的分配大小（nominal size = 不含 reserved 区域的净大小）。

**前置条件**:
- `p` 指向分配块起始地址
- `end` 指向分配块末尾地址（`p + stride - IB`）
- 分配的 header 格式合法

**后置条件**:
- 返回 `end - reserved - p`，即用户可用字节数

**编码解码规则**:
- `reserved = p[-3] >> 5` 读取 reserved 值（高 3 位）
- 若 `reserved >= 5`，则 `reserved == 5` 且实际值存储在 `*(const uint32_t *)(end-4)`
- 大 reserved 情况额外校验 `assert(!end[-5])`（零字节标记）
- 校验 `*(end - reserved) == 0`（分隔零字节）
- 校验 `*end == 0`（溢出检查字节）

**不变量**:
- 每个分配块的 `end[-reserved]` 和 `*end` 始终为零，构成内存越界写入的廉价检测

---

### get_stride

```c
static inline size_t get_stride(const struct meta *g);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 返回给定元数据所描述组中每个槽位的大小（stride = 每个 slot 的总字节数，包含 header 和 reserved 区域）。

**前置条件**:
- `g` 非 NULL

**后置条件** (Case 1: `g->last_idx == 0 && g->maplen > 0`):
- 返回 `g->maplen * 4096 - UNIT`（独立 mmap 分配，只有 1 个槽位，使用整个映射区域减去 group header）

**后置条件** (Case 2: 常规 slab 组):
- 返回 `UNIT * size_classes[g->sizeclass]`（槽位大小由 sizeclass 查表决定）

---

### set_size

```c
static inline void set_size(unsigned char *p, unsigned char *end, size_t n);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 在分配块的 in-band header 中写入用户请求大小 `n`（通过设置 reserved 区域来实现）。

**前置条件**:
- `p` 指向分配块起始
- `end` 指向分配块末尾 `p + stride - IB`
- `n <= end - p`（请求大小不大于槽位容量）

**后置条件**:
- `reserved = end - p - n`
- 若 `reserved > 0`，则 `end[-reserved] = 0`（设置分隔零字节）
- 若 `reserved >= 5`，则将 `reserved` 值以 32 位整数写入 `end[-4..-1]`，并在 `end[-5]` 置零标记
- `p[-3]` 高 3 位被设置为 `(reserved << 5)`，保留低 5 位的 slot index

**编码规则**: reserved 值使用紧凑编码。0-4 直接存于 3 位字段；5 及以上使用扩展编码，实际值存于区块末端的 4 字节字段。

---

### enframe

```c
static inline void *enframe(struct meta *g, int idx, size_t n, int ctr);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 在指定槽位中构造一个完整的新分配块。这是 `malloc()` 实际创建分配块的底层操作，包含随机化偏移计算、header 初始化等。

**前置条件**:
- `g` 非 NULL，`g->mem` 非 NULL
- `idx` 是有效的槽位索引
- `n` 是用户请求的分配大小
- `ctr` 是分配计数器（用于随机化偏移）

**后置条件**:

**Step 1 -- 计算随机化偏移**:
1. `stride = get_stride(g)` 获取槽位总大小
2. `slack = (stride - IB - n) / UNIT` 计算可用的偏移余量（以 UNIT 为单位）
3. 若原有偏移存在 (`p[-3]` 非零)，则 `off = (原偏移 + 1) & 255` 递增；否则使用 `ctr & 255`
4. 若 `off > slack`，通过 `off &= m`（m 为 slack 的掩码近似）将 off 压缩到 slack 范围内
5. 若仍超出，则 `off -= slack + 1` 取模

**Step 2 -- 设置非零偏移时的额外 header**:
- 在 `(uint16_t *)(p-2)` 存储偏移量
- `p[-3] = 7<<5` 标记（高 3 位置 1，表示使用非零偏移）
- 推进 p 到新偏移位置
- 在 `p[-4]` 设置零字节（永久校验标记）

**Step 3 -- 写入最终 header**:
- `*(uint16_t *)(p-2) = (p - g->mem->storage) / UNIT`（记录到组基址的偏移）
- `p[-3] = idx`（存储槽位索引于低 5 位）
- 调用 `set_size(p, end, n)`
- 返回 `p`

**不变量**:
- 最终 `p` 的 header 始终满足 `get_slot_index(p) == idx`
- 通过非零偏移和随机化递增，同一槽位连续分配时产生不同地址，使 double-free 更容易被 `get_meta()` 中的断言检测

**系统算法**: 使用预留区域的滑动窗口实现地址随机化，无需额外随机数生成器。偏移量循环递增（0-255 模），并在新偏移位置留下不可变的校验字节，防止攻击者伪造 header。

---

### size_to_class

```c
static inline int size_to_class(size_t n);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 将用户请求大小 `n` 映射到大小类别索引 (0..47)。

**前置条件**:
- `n` 是以字节为单位的分配大小

**后置条件**:
- 返回 0..47 的 sizeclass 索引，保证 `size_classes[sc] * UNIT >= n + IB - 1`

**系统算法**:
1. `n = (n + IB - 1) >> 4` -- 将字节大小转换为 UNIT 单位，并向上取整（补偿 IB 开销）
2. 若 `n < 10` 直接返回 `n` -- 小对象使用精确匹配（class 0-9）
3. 否则 `n++`，使用 `a_clz_32()` 计算 n 的最高位位置 `i`，结合分段查表 `size_classes[]` 精确定位
4. 通过两次比较修正索引（`i+=2` 或 `i++`）

**依赖**: `size_classes[]` (extern, 定义于 malloc.c), `a_clz_32()` (see atomic.h)

---

### size_overflows

```c
static inline int size_overflows(size_t n);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 检查请求分配大小是否会导致溢出（超过可表示的地址空间一半）。

**前置条件**:
- `n` 是以字节为单位的分配请求大小

**后置条件** (Case 1: `n >= SIZE_MAX/2 - 4096`):
- `errno = ENOMEM`
- 返回 1（溢出）

**后置条件** (Case 2: 否则):
- 返回 0（安全）

**意图说明**: `SIZE_MAX/2 - 4096` 是一个保守的溢出边界。超过此值的请求可能在内部计算 `n * 2` 或加上 group 开销时溢出。

---

### step_seq

```c
static inline void step_seq(void);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 推进全局操作序列计数器 `ctx.seq`。当计数器达上限 (255) 时，重置回 1 并清零所有 `unmap_seq[]`。

**前置条件**:
- 调用者持有 malloc 锁

**后置条件** (Case 1: `ctx.seq == 255`):
- `ctx.seq = 1`
- 所有 `ctx.unmap_seq[i] = 0` (i=0..31)

**后置条件** (Case 2: `ctx.seq < 255`):
- `ctx.seq++`

**意图说明**: 序列号用于检测特定 size class 是否在短时间内频繁 map/unmap（抖动）。每 255 次全局操作后回绕，与 `record_seq()`、`account_bounce()` 协同工作。

---

### record_seq

```c
static inline void record_seq(int sc);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 记录某个 size class 最近一次触发 unmap 时的全局序列号。

**前置条件**:
- `sc` 是 size class 索引

**后置条件**:
- 若 `sc - 7 < 32` (即 sc 在 7..38 范围内)，则 `ctx.unmap_seq[sc - 7] = ctx.seq`

**意图说明**: 仅记录 size class >= 7 的序列号，因为 class 0-6 的小对象从不触发 unmap。

---

### account_bounce

```c
static inline void account_bounce(int sc);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 检测并记录特定 size class 的 map/unmap 抖动行为。"弹跳"指同一 size class 的 group 在短时间内被反复释放后又重新分配。

**前置条件**:
- `sc` 是 size class 索引
- 调用者持有 malloc 锁

**后置条件**:
- 若 `sc - 7 < 32`，记录的上次序列号非零，且 `ctx.seq - seq < 10`（在最近 10 个全局操作内），则递增 `ctx.bounces[sc-7]`（上限 150）

**意图说明**: 抖动检测是自适应反碎片化策略的核心。当某 size class 频繁抖动时（`bounces >= 100`），分配器会延迟释放该 class 的 group，避免反复 mmap/munmap 的系统开销。

---

### decay_bounces

```c
static inline void decay_bounces(int sc);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 逐步衰减某个 size class 的弹跳计数。每次成功分配（从已有 avail 槽位）时调用，以缓慢降低对该 class 的释放惩罚。

**前置条件**:
- `sc` 是 size class 索引

**后置条件**:
- 若 `sc - 7 < 32` 且 `ctx.bounces[sc-7] > 0`，则 `ctx.bounces[sc-7]--`

**意图说明**: 衰减机制确保释放惩罚是临时性的。当抖动行为停止后，弹跳计数会随每次正常分配逐步降回 0，恢复正常释放节奏。

---

### is_bouncing

```c
static inline int is_bouncing(int sc);
```

[Visibility]: Internal -- musl mallocng 内部函数，POSIX/C 标准未定义

**意图**: 查询某个 size class 是否处于"弹跳"状态（惩罚释放的阶段）。

**前置条件**:
- `sc` 是 size class 索引

**后置条件**:
- 若 `sc - 7 < 32` 且 `ctx.bounces[sc-7] >= 100`，返回 1（弹跳中）
- 否则返回 0

**意图说明**: 阈值 100 表示近期内至少发生了 100 次快速 map/unmap 循环（减去 `decay_bounces` 的衰减）。当返回 1 时，分配器应推迟释放该 class 的空闲 group。

---

## 导出全局符号

### size_classes[]

```c
__attribute__((__visibility__("hidden")))
extern const uint16_t size_classes[];
```

[Visibility]: Internal -- musl mallocng 内部符号，通过 `glue.h` 宏重命名为 `__malloc_size_classes`。对用户程序不暴露，仅在 mallocng 的 .c 文件间共享。

**语义**: 大小类别查找表。`size_classes[i]` 表示第 i 个 size class 的槽位大小（以 UNIT 为单位）。定义于 `malloc.c`。

**约定**: 对于 sizeclass 0-9，`size_classes[i]` 精确对应大小为 i 个 UNIT 的分配。对于更大的 class，表示该 class 的最小容量。

---

### ctx

```c
__attribute__((__visibility__("hidden")))
extern struct malloc_context ctx;
```

[Visibility]: Internal -- musl mallocng 内部全局变量，通过 `glue.h` 宏重命名为 `__malloc_context`。对用户程序不暴露。

**语义**: 全局唯一的 malloc 上下文实例。定义于 `malloc.c`，在 malloc 首次调用时初始化。

**多线程安全**: 对 `ctx` 的修改必须在持有 `__malloc_lock` 下进行。读取操作（如 `get_meta` 中的断言）可能无需锁，但依赖 volatile 和原子操作保证一致性。

---

### alloc_meta()

```c
__attribute__((__visibility__("hidden")))
struct meta *alloc_meta(void);
```

[Visibility]: Internal -- musl mallocng 内部函数，通过 `glue.h` 宏重命名为 `__malloc_alloc_meta`。对用户程序不暴露。

**接口**: `struct meta *alloc_meta(void);`

**语义**: 分配一个新的 `struct meta`，优先从 `ctx.free_meta_head` 空闲链表获取，否则通过 mmap 扩展 `meta_area`。

**后置条件**: 返回一个已清零或从空闲链表中取出的 `struct meta *`。失败时程序终止（无法从内核获取内存）。

**定义位置**: `donate.c`（推测）或 `malloc.c`。详细规约见对应源文件的 spec。

---

### is_allzero()

```c
__attribute__((__visibility__("hidden")))
int is_allzero(void *);
```

[Visibility]: Internal -- musl mallocng 内部函数，通过 `glue.h` 宏重命名为 `__malloc_allzerop`。对用户程序不暴露。

**接口**: `int is_allzero(void *p);`

**语义**: 检查 `p` 指向的内存页是否全部为零。用于判断 madvise-free 后的页是否已被内核清零回收，从而决定是否需要重新 mmap。

**后置条件**:
- 返回 1: 页面全为零
- 返回 0: 页面中有非零字节

**定义位置**: `malloc.c` 或 `donate.c`。详细规约见对应源文件的 spec。

---

## 关键不变量 (跨函数全局属性)

1. **Header 自描述性**: 任何由 mallocng 返回的指针 `p` 必须满足：`p[-2]` 可解码为从 `p` 到 group 基址的偏移量，`p[-3] & 31` 可解码为槽位索引。这使得从任意指针反查元数据在 O(1) 内完成。

2. **INV-GET-META-01**: `get_meta(p)` 对任何合法分配指针必定成功返回且不与 `free()` 后的悬空指针产生误匹配。该不变量由多层 assert 保障（见 get_meta 校验链）。

3. **INV-MASK-01**: `avail_mask` 和 `freed_mask` 永不相交。即 `avail_mask & freed_mask == 0` 总是成立。

4. **INV-SLOT-COUNT-01**: 对于非 mmap 大对象组，槽位数 = `last_idx + 1`，且 `1u << last_idx` 的高位可用于区分最后一个槽位。

5. **INV-RESERVED-01**: 每个已分配块的 `end[-reserved]` 处有一个零字节作为分隔符，`*end` 处也有一个零字节作为溢出检测。这些零值在 `free()` 前不会被任何合法程序写入破坏。

6. **INV-AREA-01**: 任意有效 `struct meta *` 满足 `(((uintptr_t)meta & -4096) + offsetof(struct meta_area, check))` 处的 64 位值等于 `ctx.secret`。

7. **INV-SEQ-01**: `ctx.seq` 在 1..255 范围内循环。序列号回绕时，所有 `unmap_seq[]` 被清零，因此 `ctx.seq - unmap_seq[i]` 可正确计算两次事件之间的操作计数（无需考虑回绕）。

8. **INV-BOUNCE-01**: 对任意 size class `sc`，若 `ctx.bounces[sc-7] >= 100`，则该 class 的 group 不应立即通过 madvise/munmap 释放给内核（推迟释放以避免抖动）。

---

## 内存布局示意

```
+------------------+  <-- group base (page-aligned)
| struct meta *meta|  8 bytes (on 64-bit)
| active_idx:5     |  1 byte
| pad[11]          | 11 bytes padding
+------------------+  <-- UNIT bytes from base
| storage[0]       |  slot 0 (stride bytes)
|  ...             |
|  [IB header]     |  in-band metadata (bottom IB bytes)
+------------------+
| storage[1]       |  slot 1 (stride bytes)
|  ...             |
+------------------+
|       ...        |
+------------------+
| unused space     |  (possibly zero padding at end of mapped region)
+------------------+

  Per-slot layout:
  +----+----+----+----+----------+----+----+
  | -8 | -7 | -6 | -5 | ...data..|end-|end |
  |    |    |    |    |          | 1  |    |
  +----+----+----+----+----------+----+----+
   ^    ^    ^    ^                ^    ^
   |    |    |    |                |    +-- overflow check byte (always 0)
   |    |    |    |                +-- reserved separator (always 0)
   |    |    |    +-- optional zero check byte (for nonzero offset)
   |    |    +-- low 5 bits: slot index, high 3 bits: reserved (0-4 or 5)
   |    +-- 16-bit offset from p to group->storage (in UNITs)
   +-- optional offset storage (for nonzero initial offset, 32-bit)
```