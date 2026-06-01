# free.c 规约 (mallocng)

## 依赖图

```
free (__libc_free)
  ├── get_meta(p)                       [meta.h static inline]
  ├── get_slot_index(p)                 [meta.h static inline]
  ├── get_stride(g)                     [meta.h static inline]
  ├── get_nominal_size(p, end)          [meta.h static inline]
  ├── madvise(base, len, MADV_FREE)     [外部: sys/mman.h; glue.h 重定义为 __madvise]
  ├── wrlock()                          [glue.h static inline]
  ├── unlock()                          [glue.h static inline]
  ├── nontrivial_free(g, idx)           [free.c static] ──┐
  └── munmap(mi.base, mi.len)           [外部: sys/mman.h; glue.h 重定义为 __munmap]
                                                          │
nontrivial_free(g, i)                                     │
  ├── okay_to_free(g)                  [free.c static] ──┐│
  ├── free_group(g)                    [free.c static] ──┐││
  ├── dequeue(&ctx.active[sc], g)      [meta.h static inline]  │││
  ├── activate_group(g)                [meta.h static inline]  │││
  ├── queue(&ctx.active[sc], g)        [meta.h static inline]  │││
  ├── a_or(&g->freed_mask, self)       [atomic.h]              │││
  └── assert(sc < 48)                  [glue.h 宏]             │││
                                                               │││
okay_to_free(g)                                                │││
  ├── get_stride(g)                    [meta.h static inline]  │││
  ├── is_bouncing(sc)                  [meta.h static inline]  │││
  ├── size_classes[sc]                 [meta.h 声明; malloc.c 定义]││
  └── ctx.usage_by_class[sc]           [ctx 全局]             │││
                                                               │││
free_group(g)                                                  │││
  ├── ctx.usage_by_class[sc]            [ctx 全局]            │││
  ├── step_seq()                        [meta.h static inline]│││
  ├── record_seq(sc)                    [meta.h static inline]│││
  ├── get_meta(p)                       [meta.h static inline]│││
  ├── get_slot_index(p)                 [meta.h static inline]│││
  ├── nontrivial_free(m, idx)           [free.c static; 递归]──┘│
  └── free_meta(g)                      [meta.h static inline]──┘
```

---

## 内部数据结构

```c
struct mapinfo {
    void *base;
    size_t len;
};
```

用于在 `free()` 慢速路径中传递需要 `munmap` 归还操作系统的内存区域信息。
- `base == NULL && len == 0`: 无需 unmap。
- `base != NULL && len > 0`: 需要调用 `munmap(base, len)` 归还物理页。

此结构仅在 `mallocng/free.c` 内部使用，不暴露给其他编译单元。

---

## 类型签名与辅助宏一览

### 来自 `<meta.h>` 的 inline 函数（已包含 `assert` 校验）

| 函数 | 用途 |
|------|------|
| `struct meta *get_meta(const unsigned char *p)` | 从用户指针反查所属 group 元数据 (含校验) |
| `int get_slot_index(const unsigned char *p)` | 从指针头提取 slot 索引 (0..31) |
| `size_t get_stride(const struct meta *g)` | 计算 group 中单 slot 跨度 (含头部) |
| `size_t get_nominal_size(const unsigned char *p, const unsigned char *end)` | 校验并返回 slot 内用户数据名义大小 |
| `void free_meta(struct meta *m)` | 将 meta 结构归还到空闲链表 |
| `void queue(struct meta **phead, struct meta *m)` | 将 meta 加入环形链表 |
| `void dequeue(struct meta **phead, struct meta *m)` | 从环形链表中移除 meta |
| `uint32_t activate_group(struct meta *m)` | 激活 group：将 freed_mask 中已释放 slot 转为 avail_mask |
| `void step_seq(void)` | 递增全局序列号 (溢出则清零 unmap_seq) |
| `void record_seq(int sc)` | 记录 size class 最近 unmap 序列号 |
| `int is_bouncing(int sc)` | 判断 size class 是否处于 "弹跳" 状态 |

### 来自 `<glue.h>` 的宏

| 宏 | 含义 |
|----|------|
| `MT` | `libc.need_locks`: 多线程环境下非零，需要加锁 |
| `USE_MADV_FREE` | `0`: 编译期关闭 MADV_FREE 优化 |
| `wrlock()` | 写锁 (多线程下调用 `LOCK(__malloc_lock)`) |
| `unlock()` | 解锁 (`UNLOCK(__malloc_lock)`) |

### 来自 `<atomic.h>` 的原子操作

| 操作 | 语义 |
|------|------|
| `a_or(volatile int *p, int v)` | 原子 OR (fetch_or) |
| `a_cas(volatile int *p, int t, int s)` | 原子 CAS: 若 `*p == t` 则 `*p = s`，返回旧值 |

---

## 函数规约

---

### 1. `okay_to_free`

```c
static int okay_to_free(struct meta *g);
```

**描述**

判断一个已完全释放 (所有 slot 均 freed/available) 的 group `g` 是否应当归还操作系统 (通过 `free_group` → `munmap`)。仅由 `nontrivial_free` 在检测到 group 完全空闲时调用。

**前置条件**
- `g` 指向一个有效的 `struct meta`，其所属的所有 slot 的 `freed_mask | avail_mask == (2u<<g->last_idx) - 1`（即全部 slot 已释放或可用）。
- 调用者持有 `__malloc_lock` 写锁（在 `nontrivial_free` 中调用，锁已获取）。
- `g->sizeclass` 有效 (< 64)。

**判定逻辑 (优先级递减)**

1. **不可释放组**: 若 `!g->freeable` → 返回 `0`（保留组，后续分配复用）。
2. **大尺寸单 slot mmap (sizeclass >= 48)**: 总是返回 `1`，因为大规模 mmap 不适合 slot 复用。
3. **非标准 stride 的组**: 若 `get_stride(g) < UNIT * size_classes[sc]` → 返回 `1`，此类组无法正常放入 slot 分配体系。
4. **嵌套组 (maplen == 0)**: 组内存在另一个 group 的 slot 内 → 返回 `1`。重建开销低，且可能阻塞更大队列的释放。
5. **活跃链表中存在其他组** (`g->next != g`): → 返回 `1`。释放当前组以合并未来分配，减少碎片。
6. **非弹跳 size class**: `!is_bouncing(sc)` → 返回 `1`。非弹跳 class 的 group 可以安全释放。
7. **低容量组在高使用率弹跳 class**:
   - 计算 `cnt = g->last_idx + 1` (组内 slot 数)
   - 计算 `usage = ctx.usage_by_class[sc]` (该 class 累计分配数)
   - 若 `9*cnt <= usage && cnt < 20` → 返回 `1`。使用率足够高，说明需要更大容量的组，释放此低容量组以便后续分配新的大容量组。
8. **保底策略**: 返回 `0` —— 在弹跳 class 中保留最后一个 group 供快速复用，避免频繁 mmap/munmap 抖动。

**后置条件**
- 返回 `0`: 调用者**不会**释放该 group；`freed_mask` 将被设置，group 保留供后续 `malloc` 复用。
- 返回 `1`: 调用者将继续执行 `free_group(g)`，最终可能 `munmap` 归还内存。

**复杂度**: O(1)，纯判断逻辑。

---

### 2. `free_group`

```c
static struct mapinfo free_group(struct meta *g);
```

**描述**

释放一个 group 的全部资源。根据 group 的类型采取不同策略：
- **独立 mmap 组** (`g->maplen > 0`): 记录内存区域用于后续 `munmap`。
- **嵌套组** (`g->maplen == 0`, 嵌入在另一个 group 的 slot 中): 递归释放该 slot 所属的父 group。

**前置条件**
- `g` 是一个有效的、可以释放的 group（已通过 `okay_to_free` 判定或通过 `nontrivial_free` 条件触发）。
- 调用者持有 `__malloc_lock` 写锁。
- `g->mem->meta == g`（group 与 meta 双向关联有效）。

**处理流程**

1. **更新使用统计**: 若 `sc < 48`，`ctx.usage_by_class[sc] -= g->last_idx + 1`。
2. **独立 mmap 组路径** (g->maplen > 0):
   - `step_seq()`: 递增全局序列号 `ctx.seq`。
   - `record_seq(sc)`: 记录该 size class 最近一次 unmap 的序列号，用于弹跳检测。
   - 返回 `mapinfo { .base = g->mem, .len = g->maplen * 4096UL }`。
3. **嵌套组路径** (g->maplen == 0):
   - `p = g->mem`: 获取嵌套组基址。
   - `m = get_meta(p)`: 反查父 group 的 meta。
   - `idx = get_slot_index(p)`: 获取该 slot 在父 group 中的索引。
   - `g->mem->meta = 0`: 断开 group→meta 关联，防止悬挂指针。
   - 递归调用 `nontrivial_free(m, idx)` 释放父 group 中对应 slot。
   - 返回递归结果。
4. **回收 meta**: `free_meta(g)` 将 `g` 归还到 `ctx.free_meta_head` 空闲链表。

**后置条件**
- `g` 已被回收 (`free_meta`)，不可再访问。
- 若 `g->maplen > 0`: 返回的 `mapinfo` 包含需要 `munmap` 的内存范围。
- 若 `g->maplen == 0`: 父 group 对应 slot 已标记为 freed，返回值取决于递归路径是否需要 `munmap`。

**复杂度**: O(1) + 可能的递归 `nontrivial_free`。

---

### 3. `nontrivial_free`

```c
static struct mapinfo nontrivial_free(struct meta *g, int i);
```

**描述**

处理需要持有锁的 "非平凡" 释放操作。标记 slot `i` 为已释放，并在适当条件下对整个 group 执行释放或将其加入活跃链表。由 `free()` 慢速路径及 `free_group` 递归调用。

**前置条件**
- `g` 指向有效 `struct meta`，且 slot `i` 当前未在 `freed_mask` 或 `avail_mask` 中 (`assert(!(mask & self))` 在调用者处保证)。
- 调用者持有 `__malloc_lock` 写锁。
- `i` 在 `[0, g->last_idx]` 范围内。
- `g->sizeclass < 48`（多 slot group 的 sizeclass 范围；单 slot group 也可能经此路径但需满足特定条件）。

**处理流程**

计算 `self = 1u << i`，`mask = g->freed_mask | g->avail_mask`。

1. **全组空闲检测**:
   若 `mask + self == (2u << g->last_idx) - 1`（即本 slot 释放后组内无任何活跃分配）且 `okay_to_free(g)` 为真：
   - **出队处理** (若 group 在活跃链表中，即 `g->next != NULL`):
     - `assert(sc < 48)`: 多 slot group 必然在活跃链表。
     - 记录 `activate_new = (ctx.active[sc] == g)`。
     - `dequeue(&ctx.active[sc], g)`: 从活跃链表移除。
     - 若移除的是当前活跃 group 且链表非空，调用 `activate_group(ctx.active[sc])` 激活下一个 group。
   - 返回 `free_group(g)` 的结果。
2. **首次释放检测**:
   若 `mask == 0`（此前组内无任何 freed/available slot）：
   - `assert(sc < 48)`。
   - 若该 group 尚未在活跃链表中 (`ctx.active[sc] != g`)，则 `queue(&ctx.active[sc], g)` 将其加入链表首部。
3. **标记释放**: 无论上述条件是否满足，最终执行 `a_or(&g->freed_mask, self)` 原子设置 freed 标记。

**后置条件**
- `g->freed_mask` 的第 `i` 位必定被设置。
- 若触发全组释放: group `g` 已通过 `free_group` 回收，可能触发 `munmap`。
- 若触发首次释放: group `g` 位于 `ctx.active[sc]` 链表中。
- 返回值 `{0, 0}` (无需 unmap) 或返回 `free_group` 的结果。

**复杂度**: O(1)，不含 `free_group` 递归。

---

### 4. `free` (编译后符号: `__libc_free`)

```c
void free(void *p);
```

**描述**

musl mallocng 内存分配器的核心释放函数。释放先前由 `malloc`、`calloc` 或 `realloc` 返回的内存块 `p`。`p == NULL` 时无操作 (符合 C 标准)。

**前置条件**
- `p == NULL`，或 `p` 是由同一分配器实例先前分配的、尚未释放的有效指针。
- 分配器上下文 `ctx` 已正确初始化 (`ctx.init_done` 为真)。
- `p` 满足 16 字节对齐 (`(uintptr_t)p & 15 == 0`，`get_meta` 断言保证)。

**后置条件**
- `p` 指向的内存区域已被释放，后续对 `p` 的访问属于未定义行为。
- 若满足条件，对应物理页通过 `munmap` 归还操作系统。
- `__malloc_lock` 在函数返回时处于解锁状态。
- `errno` 在函数返回时恢复为调用前的值（`madvise` 和 `munmap` 可能修改 `errno`，已被保存/恢复）。

**逐阶段处理流程**

#### 阶段 0: NULL 快速路径

```c
if (!p) return;
```

#### 阶段 1: 元数据获取与校验

```c
struct meta *g = get_meta(p);
int idx = get_slot_index(p);
size_t stride = get_stride(g);
unsigned char *start = g->mem->storage + stride*idx;
unsigned char *end = start + stride - IB;
get_nominal_size(p, end);
```
- `get_meta(p)`: 通过 `p[-2]` (offset) 反查 `struct group *base`，再由 `base->meta` 获取 meta。执行全面校验 (offset 范围、meta 校验和、mask 一致性等)。
- `get_slot_index(p)`: 提取 `p[-3] & 31` 作为 slot 索引。
- `get_stride(g)`: 计算 slot 跨度。
- `get_nominal_size(p, end)`: 解析存储大小，校验 reserved 字段及溢出字节，**兼作内存损坏检测**。
- 前置要求 `!((uintptr_t)p & 15)`、`meta->mem == base`、`idx <= meta->last_idx`、slot 不在 freed/avail mask 中（防止 double-free）。

#### 阶段 2: 头部失效化 (双重释放检测)

```c
uint32_t self = 1u<<idx, all = (2u<<g->last_idx)-1;
((unsigned char *)p)[-3] = 255;
*(uint16_t *)((char *)p-2) = 0;
```
- `p[-3] = 255`: slot 索引字段置为无效值 31 + reserved=7，使得后续 `get_slot_index` 返回异常值。
- `*(uint16_t *)(p-2) = 0`: 清零 group 头部偏移量，使 `get_meta` 无法正确定位 group。
- 这两步确保任何对已释放指针的再释放 (double-free) 将在阶段 1 的断言校验中被捕获。

#### 阶段 3: 页粒度 MADV_FREE (已编译期禁用)

```c
if (((uintptr_t)(start-1) ^ (uintptr_t)end) >= 2*PGSZ && g->last_idx) {
    unsigned char *base = start + (-(uintptr_t)start & (PGSZ-1));
    size_t len = (end-base) & -PGSZ;
    if (len && USE_MADV_FREE) {
        int e = errno;
        madvise(base, len, MADV_FREE);
        errno = e;
    }
}
```
- 仅在 slot 跨度至少 2 个页且非单 slot group 时触发。
- 计算 slot 内完整页的起始 (`base` 对齐到页边界) 和长度 (`len` 页对齐)。
- `MADV_FREE`: 告知内核可惰性回收这些页，但在再次访问前数据仍有效。
- **`USE_MADV_FREE` 当前编译为 `0`**，此路径无实际效果。

#### 阶段 4: 快速路径 (无锁原子释放)

```c
for (;;) {
    uint32_t freed = g->freed_mask;
    uint32_t avail = g->avail_mask;
    uint32_t mask = freed | avail;
    assert(!(mask & self));       // 防止 double-free
    if (!freed || mask+self==all) break;  // 进入慢速路径
    if (!MT)
        g->freed_mask = freed+self;
    else if (a_cas(&g->freed_mask, freed, freed+self)!=freed)
        continue;                 // CAS 失败, 重试
    return;
}
```
- **进入条件**: 组内已有其他已释放 slot (`freed != 0`) 且本 slot 不是最后一个 (`mask+self != all`)。
- **单线程** (`!MT`): 直接原子写入 `g->freed_mask`。
- **多线程** (`MT`): 使用 `a_cas` (compare-and-swap) 无锁更新 `freed_mask`。若 CAS 失败 (并发修改) 则重试。
- 快速路径**避免获取全局锁**，大幅降低多线程释放竞争。

#### 阶段 5: 慢速路径 (持锁处理)

```c
wrlock();
struct mapinfo mi = nontrivial_free(g, idx);
unlock();
if (mi.len) {
    int e = errno;
    munmap(mi.base, mi.len);
    errno = e;
}
```
- `wrlock()`: 获取 `__malloc_lock` 写锁。
- `nontrivial_free(g, idx)`: 处理释放逻辑（可能释放整个 group）。
- `unlock()`: 释放锁。
- 若 `mi.len > 0`: 调用 `munmap(mi.base, mi.len)` 归还物理内存。
- `errno` 在 `munmap` 前后保存/恢复，保证释放操作不污染调用者的 `errno`。

**异常安全**
- 函数保证在返回时 `__malloc_lock` 处于解锁状态（`wrlock`/`unlock` 配对）。
- 任何内部 `assert` 失败将触发 `a_crash()` (通过 `glue.h` 的 `assert` 宏)，防止损坏状态扩散。

**复杂度**: 快速路径 O(1) 无锁；慢速路径 O(1) + 可能的 group 释放递归。

---

## 关键不变量

1. **Double-free 防护**: 阶段 2 将 `p[-3]` 设为 255、`p[-2]` 清零，使得 `get_meta` 在二次释放时断言失败。同时阶段 4 的 `assert(!(mask & self))` 也会捕获 group 内部的重复释放。

2. **锁最小化**: 快速路径 (阶段 4) 通过原子 CAS 在无锁条件下完成非首个/非最后 slot 的释放，仅在以下情况获取锁:
   - 首个释放 slot (需要将 group 加入活跃链表)
   - 最后一个释放 slot (可能需要释放整个 group)
   - 单 slot group 的释放

3. **errno 保护**: `madvise` 和 `munmap` 调用前后保存/恢复 `errno`，确保 `free()` 对 `errno` 透明。

4. **弹跳抑制 (bounce suppression)**: 通过 `ctx.bounces[sc]`、`ctx.unmap_seq[sc]`、`ctx.seq` 追踪 size class 的 unmap 频率，防止在分配/释放密集交替的模式下反复 `mmap`/`munmap`。

---

## 符号导出状态

| 符号 | 导出状态 | 说明 |
|------|---------|------|
| `free` (编译为 `__libc_free`) | **Internal (不导出给最终用户)** | 由 `glue.h` 中 `#define free __libc_free` 重命名，实际符号为 `__libc_free`。用户层 `free()` 定义于 `src/malloc/free.c`，仅为对 `__libc_free` 的薄封装。 |
| `struct mapinfo` | **Internal (不导出)** | 仅在 `free.c` 内部定义和使用。 |
| `nontrivial_free` | **Internal (static)** | 仅在 `free.c` 编译单元内可见。 |
| `free_group` | **Internal (static)** | 仅在 `free.c` 编译单元内可见。 |
| `okay_to_free` | **Internal (static)** | 仅在 `free.c` 编译单元内可见。 |

---

## 跨文件依赖

| 依赖 | 来源 | 说明 |
|------|------|------|
| `struct malloc_context ctx` | `malloc.c` (定义), `meta.h` (声明 `extern`) | 全局分配器上下文 |
| `const uint16_t size_classes[]` | `malloc.c` (定义), `meta.h` (声明 `extern hidden`) | 48 个 size class 对应的 slot 容量 (UNIT 单位) |
| `alloc_meta()` | `malloc.c` | 分配新的 meta 结构 |
| `free_meta()` | `meta.h` (static inline) | 释放 meta 结构到空闲链表 |
| `wrlock()` / `unlock()` | `glue.h` (static inline) | 基于 `__malloc_lock` 的互斥锁 |
| `a_cas()` / `a_or()` | `atomic.h` | 无锁原子操作 |