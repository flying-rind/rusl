# malloc.c 规约

## 依赖图

```
malloc (对外导出, Public)
├── size_overflows()            [inline, meta.h]
├── size_to_class()             [inline, meta.h]
├── rdlock() / wrlock() / upgradelock() / unlock()  [inline, glue.h]
├── step_seq()                  [inline, meta.h]
├── alloc_meta()                [内部, malloc.c]
│   ├── get_page_size() / PAGESIZE  [glue.h]
│   ├── get_random_secret()     [inline, glue.h]
│   ├── dequeue_head()          [inline, meta.h]
│   │   └── dequeue()           [inline, meta.h]
│   ├── brk() / mmap() / mprotect()  [syscall, glue.h]
│   └── 数据结构: struct meta_area, struct malloc_context
├── alloc_slot()                [static, malloc.c]
│   ├── try_avail()             [static, malloc.c]
│   │   ├── dequeue()           [inline, meta.h]
│   │   ├── activate_group()    [inline, meta.h]
│   │   └── decay_bounces()     [inline, meta.h]
│   ├── a_ctz_32()              [atomic.h]
│   └── alloc_group()           [static, malloc.c]
│       ├── alloc_meta()         (递归)
│       ├── alloc_slot()         (递归，用于嵌套组分配)
│       ├── free_meta()          [inline, meta.h]
│       ├── enframe()            [inline, meta.h]
│       ├── is_bouncing()        [inline, meta.h]
│       ├── account_bounce()     [inline, meta.h]
│       ├── step_seq()           [inline, meta.h]
│       ├── small_cnt_tab[][]    [static, malloc.c]
│       ├── med_cnt_tab[]        [static, malloc.c]
│       └── mmap()               [syscall, glue.h]
├── a_ctz_32()                  [atomic.h]
├── a_cas()                     [atomic.h]
└── enframe()                   [inline, meta.h]
    ├── get_stride()            [inline, meta.h]
    └── set_size()              [inline, meta.h]

is_allzero (对外导出, Internal — musl 内部辅助函数)
├── get_meta()                  [inline, meta.h]
└── get_stride()                [inline, meta.h]
```

---

## 基础常量

```
UNIT = 16      // 最小分配单元，16 字节
IB   = 4       // 带内 (in-band) 头部大小，4 字节
MMAP_THRESHOLD = 131052  // 超过此大小的分配直接使用 mmap
```

---

## 核心数据结构（定义于 meta.h）

### `struct group` — 分配组头部

[Visibility]: Internal — musl mallocng 内部数据结构，POSIX 标准未定义

```
struct group {
    struct meta *meta;          // 指向该组的元数据
    unsigned char active_idx:5; // 当前已激活的 slot 最大索引
    char pad[UNIT - sizeof(struct meta *) - 1];  // 填充至 UNIT 大小
    unsigned char storage[];    // 实际存储区起始
};
```

**设计意图**: 每个分配组是内存中一段连续的存储，组头占据一个 UNIT (16 字节)，包含指向元数据的指针和激活信息。`storage[]` 是灵活数组成员，实际存储紧接在组头之后。`active_idx` 以 5 位存储，限制最大 slot 数为 31。

---

### `struct meta` — 分配组元数据

[Visibility]: Internal — musl mallocng 内部数据结构，POSIX 标准未定义

```
struct meta {
    struct meta *prev, *next;           // 双向循环链表指针
    struct group *mem;                  // 指向关联的 group
    volatile int avail_mask, freed_mask; // 可用/已释放 slot 位掩码
    uintptr_t last_idx:5;              // 该组中最大 slot 索引
    uintptr_t freeable:1;              // 该组是否可被完全释放
    uintptr_t sizeclass:6;             // 尺寸类别 (0-47 或 63)
    uintptr_t maplen:8*sizeof(uintptr_t)-12; // mmap 分配时记录页数
};
```

**不变量**:
- `avail_mask` 和 `freed_mask` 在任意时刻交集为空：`avail_mask & freed_mask == 0`
- 对于活跃 group，`avail_mask` 中的位对应尚未分配的 slot，`freed_mask` 中的位对应已分配后又释放的 slot
- `sizeclass` 取值范围：0-47（常规）或 63（大块 mmap 分配标记）
- `last_idx` 最大值为 30（受 5 位限制），且 `2u<<last_idx` 不会溢出 32 位

---

### `struct meta_area` — 元数据区

[Visibility]: Internal — musl mallocng 内部数据结构，POSIX 标准未定义

```
struct meta_area {
    uint64_t check;             // 完整性校验值，存储 ctx.secret
    struct meta_area *next;     // 链接下一块元数据区
    int nslots;                 // 本区域可容纳的 meta 数量
    struct meta slots[];        // meta 对象数组
};
```

**不变量**:
- `check` 必须等于 `ctx.secret`，用于运行时完整性验证
- 每个 `meta_area` 占据恰好一页 (4096 字节)
- `nslots` 计算为 `(4096 - sizeof(struct meta_area)) / sizeof(struct meta)`

---

### `struct malloc_context` — 全局分配器上下文

[Visibility]: Internal — musl mallocng 内部全局状态，POSIX 标准未定义

```
struct malloc_context {
    uint64_t secret;            // 运行时随机密钥，用于校验
    size_t pagesize;            // 系统页面大小 (若未定义编译期 PAGESIZE)
    int init_done;              // 初始化完成标志
    unsigned mmap_counter;      // mmap 次数计数器，用于偏移循环
    struct meta *free_meta_head;        // 释放的 meta 空闲链表头
    struct meta *avail_meta;            // 当前可用的 meta 对象指针
    size_t avail_meta_count;            // 当前元数据区剩余 meta 数量
    size_t avail_meta_area_count;       // 剩余可用元数据页数
    size_t meta_alloc_shift;            // 元数据区分配的指数增长控制
    struct meta_area *meta_area_head;   // 元数据区链表头
    struct meta_area *meta_area_tail;   // 元数据区链表尾
    unsigned char *avail_meta_areas;    // 下一个可用元数据页地址
    struct meta *active[48];            // 48 个尺寸类别的活跃 group 链表
    size_t usage_by_class[48];          // 每个尺寸类别的 slot 使用计数
    uint8_t unmap_seq[32];             // 各尺寸类别最近 unmap 序列号
    uint8_t bounces[32];               // 反弹计数，抑制过度 mmap/unmap
    uint8_t seq;                       // 全局序列号
    uintptr_t brk;                     // 上次 brk 值，-1 表示 brk 失效
};
```

**不变量**:
- `secret` 在 `init_done` 变为 1 之后不再改变
- `active[sc]` 对应尺寸类别 `sc` 的活跃 group 循环链表（可能为空）
- `usage_by_class[sc]` 等于该尺寸类别所有活跃 group 中 slot 总数
- `avail_meta_count > 0` 时 `avail_meta` 指向有效的 meta 对象
- `meta_area_head` 和 `meta_area_tail` 构成单向链表

---

## 全局数据

### `size_classes[]`

[Visibility]: Internal — musl 内部数组，`__attribute__((__visibility__("hidden")))`，不对外导出

```c
const uint16_t size_classes[] = {
    1, 2, 3, 4, 5, 6, 7, 8,    // class 0-7:  16B-128B
    9, 10, 12, 15,              // class 8-11: 144B-240B
    18, 20, 25, 31,              // class 12-15: 288B-496B
    36, 42, 50, 63,              // class 16-19: 576B-1008B
    72, 84, 102, 127,            // class 20-23: 1152B-2032B
    146, 170, 204, 255,          // class 24-27: 2336B-4080B
    292, 340, 409, 511,          // class 28-31: 4672B-8176B
    584, 682, 818, 1023,         // class 32-35: 9344B-16368B
    1169, 1364, 1637, 2047,      // class 36-39: 18704B-32752B
    2340, 2730, 3276, 4095,      // class 40-43: 37440B-65520B
    4680, 5460, 6552, 8191,      // class 44-47: 74880B-131056B
};
```

每个元素表示该尺寸类别一个 slot 占用的 UNIT 数 (即 `UNIT*size_classes[sc]` 字节)。class 48+ 保留为大块分配使用。

---

### `small_cnt_tab[][]`

[Visibility]: Internal — `static const`，编译期常量，不对外导出

```c
static const uint8_t small_cnt_tab[][3] = {
    { 30, 30, 30 },  // class 0
    { 31, 15, 15 },  // class 1
    { 20, 10, 10 },  // class 2
    { 31, 15, 7 },   // class 3
    { 25, 12, 6 },   // class 4
    { 21, 10, 5 },   // class 5
    { 18, 8, 4 },    // class 6
    { 31, 15, 7 },   // class 7
    { 28, 14, 6 },   // class 8
};
```

每种小尺寸类别 (sc < 9) 有三个使用量等级对应的 group 内 slot 数。i=0 表示最少 slot（高使用量），i=2 表示最多 slot（低使用量）。

---

### `med_cnt_tab[]`

[Visibility]: Internal — `static const`，编译期常量，不对外导出

```c
static const uint8_t med_cnt_tab[4] = { 28, 24, 20, 32 };
```

中等尺寸类别 (sc >= 9) 的基础 slot 数，按 `sc & 3` 索引。sc%4=0 -> 28, sc%4=1 -> 24, sc%4=2 -> 20, sc%4=3 -> 32。

---

### `ctx` — 全局分配器上下文实例

[Visibility]: Internal — `__attribute__((__visibility__("hidden")))`，仅 musl 内部可见

```c
struct malloc_context ctx = { 0 };
```

整个 mallocng 分配器的全局状态，初始全零（`init_done == 0` 表示未初始化）。

---

## 内部函数规约

---

### `alloc_meta` — 分配一个元数据对象

[Visibility]: Internal — `__attribute__((__visibility__("hidden")))`，musl 内部函数

```c
struct meta *alloc_meta(void);
```

**意图 (Intent)**: 从元数据区池中分配一个新的 `struct meta` 对象。首次调用时自动初始化全局上下文（页面大小、随机密钥）。若当前元数据区耗尽，通过 `brk()` 或 `mmap()` 扩展元数据区。返回的 meta 对象 `prev` 和 `next` 字段被清零。

**前置条件**:
- 调用者需持有写锁（`wrlock`）
- `ctx` 全局可访问

**后置条件**:
- Case 1 (成功): 返回指向新分配 `struct meta` 的指针，其 `prev = next = 0`，其余字段未定义。`ctx.init_done == 1`。
- Case 2 (失败): 返回 `NULL`（`mmap` 失败或 `mprotect` 失败且 `errno != ENOSYS`）

**系统算法 (System Algorithm)**:

1. **初始化检查**: 若 `ctx.init_done == 0`，获取页面大小和随机密钥，设置 `init_done = 1`。
2. **快速路径**: 尝试从 `ctx.free_meta_head` 空闲链表队首取出 meta（调用 `dequeue_head`）。
3. **元数据区扩展**:
   - 若 `avail_meta_count == 0`：
     - 优先尝试通过 `brk()` 扩展堆顶获得新元数据页（若 `ctx.brk != -1`）。
     - 若 `brk` 不可用或失败，调用 `mmap()` 分配 `2 << meta_alloc_shift` 页。第一页设为 `PROT_NONE` 作为保护区。
     - 将新页通过 `mprotect` 设为可读写后，链接入 `meta_area_head/tail` 链表。
     - 设置 `meta_area_tail->check = ctx.secret` 用于完整性校验。
4. **返回**: `avail_meta_count--`，从 `avail_meta` 取出一个 `struct meta`，清零链表指针后返回。

**不变量**:
- 分配的每个 `meta_area` 页面大小为 4096 字节
- `meta_area->check` 始终等于 `ctx.secret`
- 若 `pagesize < 4096`，强制使用 4096

---

### `try_avail` — 尝试从 group 链表中获取可用 slot

[Visibility]: Internal — `static` 函数，不对外导出

```c
static uint32_t try_avail(struct meta **pm);
```

**意图 (Intent)**: 从 `*pm` 指向的 group 开始，沿着循环链表寻找包含可用 slot 的 group。若当前 group 无可用 slot，则遍历链表、跳过完全空闲的 group、必要时激活更多 slot。返回一个恰好设置一位的掩码，该位对应该 slot 在 group 中的索引。

**前置条件**:
- `pm` 非 NULL，`*pm` 指向一个有效的 `struct meta` 循环链表或为 NULL
- 调用者需持有读锁或写锁

**后置条件**:
- Case 1 (成功): 返回非零 `uint32_t`，`*pm` 更新为包含可用 slot 的 group，该 group 的 `avail_mask` 已更新（移除了返回的位）。若触发了 slot 激活，`active_idx` 可能已增加。
- Case 2 (失败): 返回 0，`*pm` 可能已更改（跳过已满的 group）或为 NULL。

**系统算法 (System Algorithm)**:

1. **当前 group 检查**: 读取 `m->avail_mask`，若非零则直接找到最低置位并返回。
2. **链表遍历**:
   - 若 `avail_mask == 0` 且 `freed_mask == 0`（全满且无已释放 slot）：将该 group 从链表中移出（dequeue），继续检查下一个。
   - 若 `avail_mask == 0` 但 `freed_mask != 0`（全满但有已释放 slot 可回收）：跳过到下一个 group。
3. **跳过完全空闲的 group**: 若某 group 的 `freed_mask` 覆盖了所有 slot（且 `freeable`），跳过它（避免浪费唯一活跃 group）。
4. **延迟激活**: 若 freed 的 slot 全在未激活区域（尚未被写过），跳到下一个 group。仅当这是链表中唯一的 group 时，才增加 `active_idx` 来激活更多 slot（以 4K 边界为步长增长）。
5. **激活 group**: 调用 `activate_group(m)` 将 `freed_mask` 中的可激活位转移到 `avail_mask`。
6. **反弹衰减**: 调用 `decay_bounces(m->sizeclass)` 减少该尺寸类别的反弹计数。

---

### `alloc_group` — 创建新分配组

[Visibility]: Internal — `static` 函数，不对外导出

```c
static struct meta *alloc_group(int sc, size_t req);
```

**意图 (Intent)**: 为尺寸类别 `sc` 创建一个新的分配组 (`struct meta` + `struct group`)，确定该组的 slot 数量（基于使用量启发式算法），分配存储空间（mmap 或嵌套在另一 group 中），并初始化元数据和组头。

**前置条件**:
- 调用者需持有写锁（`wrlock`）
- `0 <= sc < 48`

**后置条件**:
- Case 1 (成功): 返回指向新 `struct meta` 的指针。该 meta 的 `avail_mask` 已设置所有 slot 为可用（除首个已立即被消耗）、`freed_mask` 清零、`mem` 指向新 group、`last_idx` 和 `sizeclass` 已设置。
- Case 2 (失败): 返回 `NULL`（`alloc_meta` 失败或 `mmap` 失败，已调用 `free_meta` 归还 meta）

**系统算法 (System Algorithm)**:

1. **计算 slot 大小**: `size = UNIT * size_classes[sc]`
2. **确定 slot 数量**:
   - 对于小尺寸 (sc < 9): 根据 `usage_by_class[sc]` 在 `small_cnt_tab[sc]` 的三个等级中选择（i=0 为最少 slot）
   - 对于中等/大尺寸: 从 `med_cnt_tab[sc&3]` 基础值出发，若使用量低则减半（避免过度预分配）
   - 若 `size*cnt >= 65536*UNIT`，继续减半（slot 偏移不能超过 16 位）
   - 若 `cnt==1` 且 `size+UNIT <= pagesize/2`，增大到 2（单个 slot 且可以嵌套在另一个 group 中）
3. **大尺寸路径** (`size*cnt+UNIT > pagesize/2`):
   - 检查反弹状态 (`is_bouncing`)，更新反弹计数
   - 尝试减少 cnt 以控制浪费率（不超过当前使用量的 25%）
   - 若满足条件（低使用量、未反弹、cnt<=7），尝试将单个分配降级为独立 mmap (cnt=1)
   - 调用 `mmap` 分配整页内存
   - 计算 `active_idx`，考虑 4K 边界对齐
4. **小尺寸路径** (嵌套分配):
   - 调用 `alloc_slot(j, ...)` 在更大尺寸类别的 group 中分配空间
   - 调用 `enframe()` 初始化存储区
   - 写入特殊标记 `p[-3] = (p[-3]&31) | (6<<5)`（reserved=6 表示嵌套组）
   - 初始化所有 slot 的越界检查字节
5. **初始化元数据**: 设置 `avail_mask`、`freed_mask`、`mem->meta`、`mem->active_idx`、`last_idx`、`freeable=1`、`sizeclass=sc`
6. **更新使用量**: `ctx.usage_by_class[sc] += cnt`

---

### `alloc_slot` — 在指定尺寸类别中分配一个 slot

[Visibility]: Internal — `static` 函数，不对外导出

```c
static int alloc_slot(int sc, size_t req);
```

**意图 (Intent)**: 在尺寸类别 `sc` 中分配一个 slot。首先尝试从现有 group 中获取可用 slot，若失败则创建新的分配组。

**前置条件**:
- 调用者需持有写锁（`wrlock`）或升级锁（`upgradelock`）
- `0 <= sc < 48`

**后置条件**:
- Case 1 (成功): 返回 slot 索引 `idx` (`>= 0`)，调用者可通过 `ctx.active[sc]` 获取对应的 group
- Case 2 (失败): 返回 `-1`（`alloc_group` 失败）

**系统算法 (System Algorithm)**:

1. 调用 `try_avail(&ctx.active[sc])` 尝试从该尺寸类别的现有 group 中找到可用 slot。
2. 若 `try_avail` 成功（返回非零），使用 `a_ctz_32(first)` 将掩码转换为 slot 索引，返回该索引。
3. 若 `try_avail` 失败，调用 `alloc_group(sc, req)` 创建新 group。
4. 若 `alloc_group` 返回 NULL，返回 -1。
5. 新 group 中：`avail_mask--` 消耗首个 slot，调用 `queue()` 将新 group 加入 `ctx.active[sc]` 链表。
6. 返回索引 0（新 group 的首个 slot）。

---

### `malloc` — 分配内存

[Visibility]: Public — POSIX 标准函数，`<stdlib.h>` 声明

```c
void *malloc(size_t n);
```

**意图 (Intent)**: 分配 `n` 字节的未初始化内存。使用分尺寸类别的 group 分配策略优化常见小分配的性能和内存效率。对于大分配（>= 131052 字节），直接使用 `mmap`。

**前置条件**:
- 无特殊前置条件（分配器在首次调用时延迟初始化）
- 在多线程环境中：调用者无需持有任何锁

**后置条件**:
- Case 1 (成功): 返回指向至少 `n` 字节对齐内存的指针，内存内容未初始化。返回的指针对齐到 16 字节边界。
- Case 2 (n == 0 或溢出): 若 `n >= SIZE_MAX/2 - 4096`，`errno = ENOMEM`，返回 `NULL`。
- Case 3 (内存耗尽): 返回 `NULL`，`errno = ENOMEM`。

**系统算法 (System Algorithm)**:

1. **溢出检查**: 调用 `size_overflows(n)`，若溢出返回 NULL。

2. **大块路径** (`n >= MMAP_THRESHOLD = 131052`):
   - 计算所需页面数：`needed = n + IB + UNIT`，向上对齐到页边界
   - 调用 `mmap(0, needed, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANON, -1, 0)`
   - 获取写锁，调用 `step_seq()` 更新全局序列号
   - 分配一个 `struct meta`（通过 `alloc_meta()`）
   - 设置 meta：`sizeclass=63`（大块标记）、`maplen=页数`、`avail_mask=freed_mask=0`
   - 递增 `ctx.mmap_counter`
   - 返回 `enframe(g, 0, n, ctr)` 的结果

3. **小/中块路径** (`n < MMAP_THRESHOLD`):
   - 调用 `size_to_class(n)` 确定尺寸类别
   - **快速路径**（读锁）:
     - 获取读锁
     - 若该尺寸类别对应的偶数 sc 尚无 group 且满足条件，尝试使用粗粒度尺寸类别（sc|1），以 2 或 3 个 slot 起步而非 7 或 5
     - 循环尝试从现有 group 中通过 `avail_mask` 获取可用 slot，使用 CAS 原子操作更新掩码
     - 若成功，计算 slot 索引 `idx = a_ctz_32(first)`
   - **慢速路径**（写锁）:
     - 升级到写锁（`upgradelock()`）
     - 调用 `alloc_slot(sc, n)` 获取 slot 索引
     - 若返回 -1，释放锁并返回 NULL
   - 获取 `ctx.mmap_counter` 快照
   - 释放锁
   - 返回 `enframe(g, idx, n, ctr)` 的结果

**并发安全**:
- 读锁用于 fast-path：多个线程可同时在不同尺寸类别尝试分配
- 写锁用于 slow-path：创建新 group 时需要独占访问
- CAS 原子操作用于 `avail_mask` 的无锁竞争更新
- `mmap_counter` 的快照在锁外获取，但不影响正确性（仅用于地址随机化）

**不变量**:
- 所有从 `malloc` 返回的指针对齐到 16 字节
- 分配的内存块前 4 字节 (IB) 为带内头部，包含 slot 索引和保留大小
- 通过 `get_meta()` 可反向从指针推导出所属的 `struct meta` 和 `struct group`

---

### `is_allzero` — 判断分配块是否全部为零

[Visibility]: Internal — `__attribute__((__visibility__("hidden")))`，musl 内部辅助函数，用于 `calloc` 优化

```c
int is_allzero(void *p);
```

**意图 (Intent)**: 判断指针 `p` 指向的已分配内存块是否可以被视为全部为零，从而在 `calloc` 实现中跳过显式的 `memset`。该优化适用于来自 `mmap` 全新分配或来自 fresh OS 页面的内存块。

**前置条件**:
- `p` 必须是 `malloc` 返回的有效指针
- `p` 对齐到 16 字节

**后置条件**:
- Case 1 (全部为零): 返回 1 — 该块来自 sizeclass >= 48 的大块 mmap 分配，或该块的 stride（真实大小）小于其名义尺寸（即该块内含有越界填充区域，意味着该 slot 未被完全使用/未被写过）
- Case 2 (可能非零): 返回 0 — 该块可能包含先前释放留下的脏数据

**系统算法 (System Algorithm)**:

1. 调用 `get_meta(p)` 获取关联的 `struct meta`
2. 若 `g->sizeclass >= 48`，返回 1（大块 mmap 分配，OS 已将页面清零）
3. 若 `get_stride(g) < UNIT*size_classes[g->sizeclass]`，返回 1（组内 slot stride 小于标准尺寸，说明该 slot 对应的内存从未被有效使用过）
4. 否则返回 0

---

## 全局不变量 (Global Invariants)

以下不变量适用于 mallocng 分配器的整个生命周期：

1. **元数据完整性**: 每个 `meta_area->check` 必须等于 `ctx.secret`。通过 `get_meta()` 验证时检查此条件。

2. **Group-元数据双向链接**: 对于活跃 group，`g->mem->meta == g` 始终成立。

3. **Slot 索引范围**: 任何已分配 slot 的索引 `idx` 满足 `idx <= meta->last_idx`。

4. **位掩码一致性**: 对于任意 group，slot 索引 `i` 不可能同时在 `avail_mask` 和 `freed_mask` 中，即 `!(avail_mask & (1u<<i) & freed_mask)`。

5. **尺寸类别范围**: `meta->sizeclass` 的取值范围为 0-47（常规分配）或 63（大块 mmap 分配）。

6. **锁层级**: 读锁和写锁是同一个锁（`__malloc_lock`），区别仅在于调用约定 — 读锁用于 fast-path 的"乐观读取"，写锁用于 slow-path 的排他写入。实际上 `rdlock`、`wrlock` 的实现相同，都是由 `LOCK(__malloc_lock)` 实现，这意味着在 musl 的实现中实际上总是排他锁，但协议保留了读/写锁的语义以供未来优化。

7. **Active 链表**: `ctx.active[sc]` 是循环双向链表，或为 NULL（空链表）。该链表中的每个 group 至少有一个 `avail_mask` 或 `freed_mask` 中的可用 slot（group 被移出链表意味着它已满）。

8. **使用量统计**: `ctx.usage_by_class[sc]` 等于该尺寸类别所有活跃 group 中 `last_idx+1` 之和。

---

## 跨文件依赖说明

本文件实现所依赖的其他模块接口：

| 依赖符号 | 来源文件 | 说明 |
|---------|---------|------|
| `size_to_class()` | `meta.h` (inline) | 将字节数转换为尺寸类别索引 |
| `size_overflows()` | `meta.h` (inline) | 检查分配大小是否溢出 |
| `enframe()` | `meta.h` (inline) | 在 group 中初始化一个 slot 供用户使用 |
| `get_meta()` | `meta.h` (inline) | 从用户指针反查 struct meta |
| `get_stride()` | `meta.h` (inline) | 获取 group 中每个 slot 的步长 |
| `set_size()` | `meta.h` (inline) | 在 slot 头部写入名义大小和保留大小 |
| `activate_group()` | `meta.h` (inline) | 原子地将 freed_mask 中的可激活 slot 移至 avail_mask |
| `queue()` / `dequeue()` / `dequeue_head()` | `meta.h` (inline) | 双向循环链表操作 |
| `free_meta()` | `meta.h` (inline) | 将 meta 归还到空闲链表 |
| `step_seq()` | `meta.h` (inline) | 更新全局序列号 |
| `account_bounce()` | `meta.h` (inline) | 更新反弹计数器 |
| `decay_bounces()` | `meta.h` (inline) | 衰减反弹计数 |
| `is_bouncing()` | `meta.h` (inline) | 检查尺寸类别是否处于反弹状态 |
| `record_seq()` | `meta.h` (inline) | 记录尺寸类别的 unmap 序列号 |
| `get_random_secret()` | `glue.h` (inline) | 生成运行时随机密钥 |
| `rdlock()` / `wrlock()` / `unlock()` / `upgradelock()` | `glue.h` (inline) | 锁操作 |
| `a_ctz_32()` | `atomic.h` | Count trailing zeros (计数末尾零) |
| `a_cas()` | `atomic.h` | Compare-and-swap (比较并交换) |
| `a_crash()` | `atomic.h` | 断言失败时崩溃 |
| `brk()` / `mmap()` / `mprotect()` / `munmap()` | `glue.h` (宏) | 系统调用封装 |
| `free_group()` | `free.c` (static) | 释放整个 group（见 free.c spec） |