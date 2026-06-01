# free.c 规约 (mallocng 实现)

> **实现架构说明**：`src/malloc/free.c` 仅定义了一层薄封装，将 POSIX `free(void *p)` 转发给内部实现 `__libc_free`。实际算法位于 `src/malloc/mallocng/free.c`，后者通过 `glue.h` 中的 `#define free __libc_free` 将本地 `free` 函数重命名导出为 `__libc_free`。本规约涵盖完整依赖链。

---

## 依赖图

```
free (POSIX, src/malloc/free.c)
  └── __libc_free (即 mallocng free, src/malloc/mallocng/free.c)
        ├── get_meta(p)               [meta.h inline] — 从指针恢复 struct meta
        ├── get_slot_index(p)         [meta.h inline] — 从指针提取槽位索引
        ├── get_stride(g)             [meta.h inline] — 获取组的步长
        ├── get_nominal_size(p, end)  [meta.h inline] — 验证并返回分配大小
        ├── nontrivial_free(g, idx)   [本文件 static] — 慢路径释放
        │     ├── okay_to_free(g)     [本文件 static] — 判断是否应释放整组
        │     ├── free_group(g)       [本文件 static] — 释放整个组
        │     │     ├── step_seq()         [meta.h inline]
        │     │     ├── record_seq(sc)     [meta.h inline]
        │     │     ├── get_meta(p)        [meta.h inline]
        │     │     ├── get_slot_index(p)  [meta.h inline]
        │     │     ├── nontrivial_free()  [递归]
        │     │     └── free_meta(g)       [meta.h inline]
        │     ├── queue() / dequeue()  [meta.h inline] — 活动链表操作
        │     └── a_or()               [atomic.h] — 原子位设置
        ├── wrlock() / unlock()       [glue.h inline] — 锁操作
        ├── madvise(MADV_FREE)        [Linux syscall] — 页面回收提示
        └── munmap()                  [Linux syscall] — 解除内存映射
```

---

## Level 1: 简单规约 (Public API)

### free (对外导出, src/malloc/free.c)

```c
void free(void *p);
```

**[Visibility]: Public — C 标准函数，`<stdlib.h>` 声明，POSIX 标准要求**

- **前置条件 (Precondition)**:
  - `p` 必须是之前由 `malloc`、`calloc`、`realloc`、`aligned_alloc` 或 `posix_memalign` 返回的有效指针，**或**为 `NULL`
  - 若 `p` 非空，其指向的内存必须尚未被释放（double-free 会导致未定义行为）
  - 调用者不持有任何 `malloc` 相关的内部锁（本函数内部自行处理同步）

- **后置条件 (Postcondition)**:
  - **Case 1: `p == NULL`**: 函数立即返回，无任何操作。这是 C 标准要求的无操作行为。
  - **Case 2: `p != NULL`**: 指针指向的内存被标记为可供后续分配重用。释放后 `p` 自身的值不变，但变为悬垂指针，再次解引用或释放均为未定义行为。

- **错误处理**: 无返回值，不设置 `errno`（C 标准规定 `free()` 不报告错误）

- **线程安全**: 完全线程安全。通过内部锁 (`__malloc_lock`) 保护全局分配器状态，并在 fast-path 路径上使用无锁 CAS 原子操作优化高并发场景。

- **信号安全**: 不是 async-signal-safe。持有锁期间被信号中断可能导致死锁。

- **Intent**: 将一块动态分配的内存归还给分配器，使其可被后续 `malloc` 调用重用。实现采用分层策略：fast-path 原子释放（无锁）处理同组内非首/非末释放；slow-path 加锁处理边界情况（首/末释放触发整组回收、mmap 解除映射等）。

---

## Level 3: 高度优化设计 (内部实现, 系统算法)

### __libc_free (即 mallocng free, 内部符号)

```c
void free(void *p);  // 通过 glue.h 的 #define free __libc_free 导出为 __libc_free
```

**[Visibility]: Internal — musl 内部符号。`__libc_free` 是为 `__libc_` 前缀的 libc 内部函数，C 标准和 POSIX 均未定义此符号。用户程序通过 `stdlib.h` 使用无前缀的 `free()`。**

- **前置条件**: 同 public `free`，但此外还要求 musl 的 malloc 子系统已完成初始化（首次调用 malloc/calloc/realloc 时会自动初始化，详见 `malloc.c` spec）

- **后置条件**: 同 public `free`

- **意图 (Intent)**: musl mallocng 分配器的核心释放逻辑。采用三级处理路径：
  1. **Fast-path（无锁原子释放）**: 若释放的槽位不是组内首/末活跃槽位，直接原子 CAS 更新 `freed_mask`，零锁竞争
  2. **Slow-path（加锁释放）**: 若释放触发组边界条件（最后一个被使用槽位或首个释放），加锁调用 `nontrivial_free` 执行复杂的组管理逻辑
  3. **Page-level reclamation**: 在释放前，对大槽位中完整的空闲物理页通过 `madvise(MADV_FREE)` 向内核提示可回收

- **系统算法 (System Algorithm)**:

  **第一阶段: 元数据定位与验证**

  从用户指针 `p` 恢复分配器内部元数据。musl mallocng 的独特设计是使用指针前 4 字节（`IB = 4`）作为 out-of-band header：
  ```
  [group header (UNIT bytes)]
  [slot 0: ...]
  [slot 1: ...]
  ...
  对于每个槽位中分配的块:
    p-4: 溢出标志字节 (0 或非零)
    p-3: bit[4:0] = slot index; bit[7:5] = reserved size
    p-2,p-1: uint16_t offset from group base (以 UNIT=16 为单位)
    p:    用户数据起始
  ```

  算法步骤：
  1. `g = get_meta(p)`: 从 `p[-2]` 读取到组基址的偏移量（2 字节或 4 字节大偏移），定位 `struct group`，再通过 `group->meta` 获取 `struct meta`。同时验证 `meta_area->check == ctx.secret`（防 corruption）
  2. `idx = get_slot_index(p)`: 提取 `p[-3] & 31` 作为槽位索引
  3. `stride = get_stride(g)`: 对于 mmap 单槽组，stride = maplen*4096 - UNIT；否则 stride = UNIT*size_classes[sc]
  4. 计算 `start = g->mem->storage + stride*idx` 和 `end = start + stride - IB`
  5. `get_nominal_size(p, end)`: 验证槽位的 reserved 字段和溢出字节，确认数据完整性

  **第二阶段: 写防护字节（double-free 检测辅助）**

  ```
  p[-3] = 255;              // 标记为无效 (slot index = 31, reserved = 7, 均为最大非法值)
  *(uint16_t *)(p-2) = 0;   // 清零组内偏移，使后续 get_meta 在此指针上必然失败
  ```

  **第三阶段: 页面回收提示（MADV_FREE / lazy freeing）**

  ```
  条件: ((uintptr_t)(start-1) ^ (uintptr_t)end) >= 2*PGSZ && g->last_idx > 0
  ```
  即：当槽位跨越至少 2 个物理页边界，且组有多个槽位（非单槽 mmap 组，因为单槽组即将被整组 unmap）时：
  1. 计算槽位内完整的物理页范围（对齐到 PGSZ）
  2. 若 `USE_MADV_FREE` 为真（当前为 0，默认禁用），调用 `madvise(base, len, MADV_FREE)` 告知内核可惰性回收这些页面
  3. 保存并恢复 `errno`（`madvise` 可能修改 errno，C 标准要求 `free()` 不改变 errno）

  **第四阶段: Fast-path 原子释放（无锁）**

  进入原子无锁循环，竞争条件通过 CAS 解决：

  ```
  for (;;) {
      freed = g->freed_mask;
      avail = g->avail_mask;
      mask = freed | avail;
      assert(!(mask & self));  // self 位不得已在 freed 或 avail 中（防止 double-free）
      if (!freed || mask+self == all) break;  // 首个释放 或 最后一个被使用槽位 → slow path
      if (!MT)
          g->freed_mask = freed + self;        // 单线程：直接写入
      else if (a_cas(&g->freed_mask, freed, freed+self) != freed)
          continue;                             // CAS 失败，重试
      return;                                   // CAS 成功，释放完成
  }
  ```

  关键洞察：fast-path 的数学条件 `!freed || mask+self == all` 意味：
  - 如果 `freed_mask == 0`（组内此前无释放），这是"首释"→ 必须 slow-path 判断是否需要 activate group
  - 如果 `mask + self == all`（释放此槽位后所有槽位均 freed/avail，即成"末释"）→ 必须 slow-path 判断是否需要 free_group

  **第五阶段: Slow-path 加锁释放**

  ```
  wrlock();
  mi = nontrivial_free(g, idx);
  unlock();
  if (mi.len) {
      e = errno;
      munmap(mi.base, mi.len);
      errno = e;
  }
  ```

  锁操作：`wrlock()` 在 `MT`（多线程模式）下通过 `LOCK(__malloc_lock)` 获取自旋锁；`unlock()` 释放。非 MT 模式下为空操作。

  若 `nontrivial_free` 返回需要 `munmap` 的映射范围，执行 `munmap` 并保护 `errno`。

---

### nontrivial_free (内部 static 函数)

```c
static struct mapinfo nontrivial_free(struct meta *g, int i);
```

**[Visibility]: Internal — static 函数，仅在本编译单元内可见。musl mallocng 内部释放逻辑的核心分支。**

- **前置条件**:
  - 必须在持有 `__malloc_lock` 写锁的情况下调用
  - `g` 指向有效的 `struct meta`，其 `mem` 指向的 `struct group` 存在
  - `i` 是待释放槽位在组内的索引，满足 `0 <= i <= g->last_idx`
  - 槽位 `i` 当前未被标记为 freed 或 avail（`!(g->freed_mask & (1<<i)) && !(g->avail_mask & (1<<i))`）

- **后置条件**:
  - **Case 1: 整组释放**: 当 `mask + self == all`（释放此槽位后组内无活跃槽位）且 `okay_to_free(g)` 返回真时：
    - 对于多槽组（sc < 48）：从 `ctx.active[sc]` 链表中 dequeue，若被移除的是当前 active 组，激活链表中的下一个组
    - 调用 `free_group(g)` 回收整组资源
    - 返回需要 `munmap` 的 `mapinfo`（若组是 mmap'd）或零值
  - **Case 2: 标记为 avail 并重新入队**: 当此前组内无任何 freed/avail 槽位（`mask == 0`），且组不在 active 链表上时（`ctx.active[sc] != g`）：
    - 将组 `queue` 到 `ctx.active[sc]` 链表，标记为可复用
    - 设置 `g->freed_mask |= self`
  - **Case 3: 仅标记释放**: 其他情况下，仅通过 `a_or(&g->freed_mask, self)` 原子设置释放位

- **Intent**: 决定释放后的组管理策略。核心状态机：
  - 若释放导致组"全空"且策略允许，则彻底回收整组
  - 若组"首次出现空闲槽位"且当前未被 active 追踪，将其加入活动链表供后续分配
  - 否则仅标记该槽位为 freed，等待同组其他槽位释放

- **系统算法**: 使用位掩码管理槽位状态：
  ```
  self = 1u << i                          // 本槽位的位掩码
  mask = g->freed_mask | g->avail_mask     // 已有空闲/可用槽位
  all  = (2u << g->last_idx) - 1           // 所有槽位的掩码
  
  条件 A: mask + self == all → 释放后组全空
  条件 B: mask == 0          → 组内此前无空闲槽位
  ```

---

### free_group (内部 static 函数)

```c
static struct mapinfo free_group(struct meta *g);
```

**[Visibility]: Internal — static 函数，仅在本编译单元内可见。负责释放整个 slot group 的所有资源。**

- **前置条件**:
  - 必须在持有 `__malloc_lock` 写锁的情况下调用
  - `g` 指向有效的 `struct meta`
  - 组内所有槽位已确认无活跃分配（调用者已做此判断）
  - 若 `g->next` 和 `g->prev` 非零，说明该组在某个链表上，调用者必须已将其 dequeue

- **后置条件**:
  - `struct meta` 通过 `free_meta(g)` 归还给 `ctx.free_meta_head` 空闲链表
  - 若 `g->sc < 48`：更新 `ctx.usage_by_class[sc]` 减去该组贡献的槽位数
  - **Case 1: mmap 组 (`g->maplen > 0`)**:
    - 调用 `step_seq()` / `record_seq(sc)` 记录 unmap 序列号（用于 bounce 检测）
    - 返回 `{base: g->mem, len: g->maplen*4096}` → 调用者执行 `munmap`
  - **Case 2: 子分配组 (`g->maplen == 0`)**:
    - 该组是作为"大槽位内的子组"分配的
    - 将 `g->mem->meta` 置 0（标记该 group header 不再有效）
    - **递归调用** `nontrivial_free(m, idx)` 释放父组中对应的槽位
    - 返回父组释放产生的 `mapinfo`（由递归调用返回）

- **Intent**: 将一组槽位所占用的全部资源归还系统或父分配器。关键设计决策：
  - mmap 组直接 `munmap` 归还内核，减少进程 RSS
  - 子分配组递归归还给父组槽位，父组槽位重新变为可用

---

### okay_to_free (内部 static 函数)

```c
static int okay_to_free(struct meta *g);
```

**[Visibility]: Internal — static 函数，仅在本编译单元内可见。实现"bounce prevention"启发式策略，防止在特定大小类上出现分配/释放抖动。**

- **前置条件**:
  - 必须在持有 `__malloc_lock` 写锁的情况下调用
  - `g` 指向有效的 `struct meta`，且组内所有槽位均已释放或即将变为可用

- **后置条件**:
  - 返回 0 或 1，不修改任何全局状态
  - 返回值指示是否应该释放该组（1 = 释放，0 = 保留）

- **Intent**: 在线分配器中的关键启发式——阻止"bouncing"（抖动），即某个大小类频繁分配后又立即释放整组，导致反复 mmap/munmap。策略通过 `ctx.bounces[]` 数组追踪各大小类近期 unmap 频率，对抖动类保守地保留至少一个组。

- **系统算法 (7 层决策级联)**:

  ```
  (1) if (!g->freeable) return 0;
      → 显式标记不可释放的组（如 donate 产生的组）

  (2) if (sc >= 48 || get_stride(g) < UNIT*size_classes[sc]) return 1;
      → 大对象（>= MMAP_THRESHOLD）或步长不匹配的组总是释放

  (3) if (!g->maplen) return 1;
      → 子分配组（嵌入在父组槽位中的组）总是释放
      → 原因：重建成本低，且可能阻塞父组的大槽位回收

  (4) if (g->next != g) return 1;
      → 若有另一个非满组，释放此组以减少碎片、合并分配

  (5) if (!is_bouncing(sc)) return 1;
      → 非抖动大小类，直接释放

  (6) if (9*cnt <= usage && cnt < 20) return 1;
      → 即便在抖动类中，若使用量高而组槽位数少，释放低容量组以推动创建更优组
      → cnt = g->last_idx+1（组槽位数），usage = ctx.usage_by_class[sc]

  (7) return 0;
      → 保底：抖动类中保留最后一个组，防止反复 mmap/munmap
  ```

  **Bounce 检测机制**（`is_bouncing` / `record_seq` / `decay_bounces`）:
  - 全局序列号 `ctx.seq` 递增（0..255 循环），每次 size class 7..38 的 munmap 发生时记录 `ctx.unmap_seq[sc-7] = seq`
  - `account_bounce(sc)`: 若距上次 unmap < 10 个序列号窗口，递增 `ctx.bounces[sc-7]`（上限 150）
  - `decay_bounces(sc)`: 每次成功在该类分配时递减 bounce 计数
  - `is_bouncing(sc)`: `bounces[sc-7] >= 100` 表示该类正在抖动
  - 此机制类似 TCP 拥塞控制的 AIMD 思想，用序列号窗口替代时间窗口，避免 syscall 开销

---

## 不变量 (Invariants)

### I1: errno 保持不变量
`free()` 的执行（包括内部 `madvise` 和 `munmap` syscall）**必须保证调用者的 `errno` 值不被改变**。任何可能修改 `errno` 的 syscall 前后必须保存/恢复 `errno`。

### I2: 元数据完整性不变量
在任何线程释放操作前后，以下性质始终成立：
- 若 `p` 是有效的已分配指针，则 `get_meta(p)` 能成功定位到正确的 `struct meta`，且 `meta_area->check == ctx.secret`
- 一个槽位不能同时出现在 `freed_mask` 和 `avail_mask` 中
- `ctx.active[sc]` 链表上的每个组，其 `avail_mask` 必须非零（即组内有可用槽位）

### I3: 锁层级不变量
- `nontrivial_free`、`free_group`、`okay_to_free` 必须在持有 `__malloc_lock` 写锁时调用
- fast-path 原子 CAS 路径不持有锁，通过 compare-and-swap 保证 `freed_mask` 更新的原子性

### I4: usage_by_class 一致性不变量
`ctx.usage_by_class[sc]` 应等于所有 sc 类的活动组中 `last_idx+1` 的和。当组被 `free_group` 释放时，其贡献从计数中扣减。

### I5: 单槽 mmap 组不变量
当 `g->last_idx == 0 && g->maplen > 0`（单槽 mmap 组）时，释放该槽位必定触发整组 `munmap`，因此不会走 `madvise(MADV_FREE)` 页面回收路径。

---

## 相关数据结构

### struct mapinfo (内部类型)

```c
struct mapinfo {
    void *base;
    size_t len;
};
```

**[Visibility]: Internal — 仅在 `mallocng/free.c` 内定义，用于在 `nontrivial_free` 和调用者之间传递需要 unmap 的内存范围信息。零值 `{0, 0}` 表示无需 unmap。**

用于在 `free_group` → `nontrivial_free` → `free` 的调用链中传递需要 `munmap` 的地址和长度。

---

## 关键内联辅助函数 (来自 meta.h / glue.h，算法组件说明)

以下函数均为 `static inline`，是 `__libc_free` 实现逻辑的组成部分。此处仅描述其在 free 路径中的角色，完整规约见 `meta.h` 和 `glue.h` 对应的 spec 文件。

| 函数 | 角色 |
|------|------|
| `get_meta(p)` | 将用户指针反查为 `struct meta`，同时执行全方位完整性断言 |
| `get_slot_index(p)` | 提取 `p[-3] & 31` 作为槽位索引 |
| `get_stride(g)` | 返回组内每个槽位的字节步长 |
| `get_nominal_size(p, end)` | 从 reserved 字段恢复原始分配大小并验证溢出守卫字节 |
| `free_meta(m)` | 将 meta 清零并归还到 `ctx.free_meta_head` 空闲链表 |
| `queue(head, m)` / `dequeue(head, m)` | 双向循环链表操作，维护 `ctx.active[sc]` |
| `activate_group(m)` | 原子地将 `freed_mask` 中已确认释放的槽位转移到 `avail_mask`，使组重新可分配 |
| `step_seq()` | 递增全局序列号（0..255 循环），用于 bounce 检测的时间窗口 |
| `record_seq(sc)` | 记录某大小类最近一次 unmap 的序列号 |
| `is_bouncing(sc)` | 查询某大小类是否处于抖动状态 |
| `size_to_class(n)` | 将字节大小映射到 size class 索引 |
| `wrlock()` / `unlock()` | 多线程写锁获取/释放（`MT` 模式下通过 `LOCK(__malloc_lock)`） |

---

## 调用者（外部模块依赖）

| 调用者 | 说明 |
|--------|------|
| `realloc(p, 0)` | 当 realloc 的 size 为 0 时等价于 `free(p)`，最终调用 `__libc_free` |
| `__libc_free` 的直接调用者 | 其他 libc 内部模块需直接释放内存时使用（如 atexit 清理、stdio 缓冲区释放等） |

## 被调用者（free 依赖的外部模块）

| 被调用者 | 来源 | 说明 |
|----------|------|------|
| `madvise(base, len, MADV_FREE)` | Linux kernel | 惰性页面回收提示（当前 `USE_MADV_FREE=0`，编译期禁用） |
| `munmap(base, len)` | Linux kernel | 释放 mmap 分配的内存映射 |
| `a_cas(p, t, s)` | `atomic.h` | compare-and-swap 原子操作 |
| `a_or(p, v)` | `atomic.h` | 原子按位或 |
| `LOCK(m)` / `UNLOCK(m)` | `lock.h` | 自旋锁（futex-based） |

---

## 内存布局图

```
  struct meta_area (PGSZ-aligned)
  ┌─────────────────────────────┐
  │ check (= ctx.secret)        │
  │ next                        │
  │ nslots                      │
  │ struct meta slots[0]        │ ← 被 free 路径通过 get_meta 间接引用
  │ struct meta slots[1]        │
  │ ...                         │
  └─────────────────────────────┘
           │
           │  struct meta
           ▼
  ┌─────────────────────────────┐
  │ prev, next (active list)    │
  │ mem ──────────────────────┐ │
  │ avail_mask, freed_mask    │ │
  │ last_idx:5, freeable:1,   │ │
  │ sizeclass:6, maplen:N     │ │
  └─────────────────────────────┘
                                │
           struct group         ▼
  ┌─────────────────────────────┐  ← g->mem
  │ meta ───────────────────────┤
  │ active_idx:5, pad[...]      │
  │ storage[0..stride*0-1]      │  ← slot 0
  │ storage[stride*0..]         │
  │   [IB bytes] p[-4..-1]      │  ← out-of-band header
  │   [n bytes]  user data      │  ← p (user pointer)
  │   [reserved] overflow guard │
  │ storage[stride*1..]         │  ← slot 1
  │ ...                         │
  └─────────────────────────────┘
```

## 安全考虑

1. **Double-free 检测**: `get_meta` 中的 `assert(!(meta->avail_mask & (1u<<index)))` 和 `assert(!(meta->freed_mask & (1u<<index)))` 基于掩码检测 double-free。fast-path 中的 `assert(!(mask&self))` 提供早期检测。此外，free 后将 `p[-3]=255` 和 `*(uint16_t*)(p-2)=0` 使二次释放时 `get_meta` 必然失败（`assert(!((uintptr_t)p & 15))` 或后续校验失败）。

2. **元数据 corruption 检测**: `meta_area->check == ctx.secret` 验证 meta area 完整性。`get_meta` 中的多重断言（offset 范围、size class 一致性、maplen 边界）提供深度防御。

3. **errno 保持**: 所有可能修改 `errno` 的 syscall（`madvise`、`munmap`）前后均有保存/恢复操作，满足 C 标准要求。