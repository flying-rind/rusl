# malloc_usable_size.c 规约

> 所在模块：`src/malloc/mallocng/` (musl 下一代内存分配器)

---

## 依赖图

```
malloc_usable_size (Public)
  ├── get_meta(p)            [meta.h, static inline]
  │     ├── get_slot_index(p) [meta.h, static inline]
  │     ├── struct meta       [meta.h]
  │     ├── struct group      [meta.h]
  │     ├── struct meta_area  [meta.h]
  │     ├── ctx               [meta.h, 全局上下文]
  │     ├── size_classes[]    [meta.h, 导出但 hidden]
  │     ├── UNIT              [meta.h, 宏]
  │     └── IB                [meta.h, 宏]
  ├── get_slot_index(p)       [meta.h, static inline]
  ├── get_stride(g)           [meta.h, static inline]
  │     ├── size_classes[]    [meta.h]
  │     └── UNIT              [meta.h, 宏]
  └── get_nominal_size(p, end) [meta.h, static inline]
```

---

## 关键数据结构与常量

### `UNIT` 与 `IB` (内部宏)

```
[Visibility]: Internal — musl mallocng 内部宏，POSIX/C 标准未定义
```

```c
#define UNIT 16   // 基本分配单元（16 字节）
#define IB 4      // 每个 slot 末尾预留的"带内"(In-Band)元数据字节数
```

**语义**：
- `UNIT` 是 mallocng 内部地址对齐与大小量化的基本粒度。所有分配大小向上取整到 UNIT 的倍数。
- `IB` 是每个 slot 末尾固定预留的控制字节数。这 4 字节存储溢出检测标记(`overflow byte`)和保留大小信息，不归用户可用空间。

**不变量**：`UNIT >= IB`，即基本分配单元不小于带内元数据大小。

---

### `struct group` (内部结构体)

```
[Visibility]: Internal — musl mallocng 内部数据结构，不对外暴露
```

```c
struct group {
    struct meta *meta;          // 指向所属的 meta 控制块
    unsigned char active_idx:5; // 组内当前活跃的最大 slot 索引（0-31）
    char pad[UNIT - sizeof(struct meta *) - 1]; // 填充至 UNIT 对齐
    unsigned char storage[];    // 柔性数组，实际存储区域
};
```

**语义**：
- `group` 是一组固定大小 slot 的容器。一个 group 恰好占据一页(4096 字节)或由 `maplen` 指定的大页。
- `meta` 指针回指到该 group 的元数据控制块，形成双向关联。
- `storage[]` 是实际分配给用户的字节存储区。

**不变量**：
- `&group == &group->meta->mem` —— group 地址与其 meta 中的 `mem` 指针严格相等。
- `sizeof(struct group) == UNIT`，即 group 头部恰好占 16 字节。

---

### `struct meta` (内部结构体)

```
[Visibility]: Internal — musl mallocng 内部元数据结构，不对外暴露
```

```c
struct meta {
    struct meta *prev, *next;     // 双向链表指针（用于 active/free 队列）
    struct group *mem;            // 指向对应的 group
    volatile int avail_mask;      // 位掩码：可分配的 slot
    volatile int freed_mask;      // 位掩码：已释放待回收的 slot
    uintptr_t last_idx:5;         // 该 group 中最大的 slot 索引
    uintptr_t freeable:1;         // 该 group 是否可整体释放
    uintptr_t sizeclass:6;        // 大小类别 (0-47 为常规类, 63 为 mmap 大块)
    uintptr_t maplen:8*sizeof(uintptr_t)-12; // mmap 分配时的页数
};
```

**语义**：
- `avail_mask`：位 i 为 1 表示 slot i 可分配（空闲），为 0 表示不可分配（已占用或尚未激活）。
- `freed_mask`：位 i 为 1 表示 slot i 已被 free() 释放但尚未重新加入 avail_mask。
- `sizeclass` 为 63 表示这是 mmap 分配的大块（无固定 slot 大小），0-47 为预定义的 48 个大小类别。
- `maplen` 仅在 mmap 分配时有意义，表示分配的页数。
- 位域总大小恰好等于 `sizeof(uintptr_t)`，与 `prev`/`next` 指针对齐。

---

### `struct meta_area` (内部结构体)

```
[Visibility]: Internal — musl mallocng 内部结构，不对外暴露
```

```c
struct meta_area {
    uint64_t check;           // 安全检查值（等于 ctx.secret）
    struct meta_area *next;   // 链表指针
    int nslots;               // 该区域中 meta slot 的数量
    struct meta slots[];      // 柔性数组：meta 对象存储区
};
```

**语义**：`meta_area` 是一页大小的区域，用于批量存储 `struct meta` 对象。每个 meta_area 的 `check` 字段必须等于 `ctx.secret`，构成一种轻量级完整性校验。

---

### `ctx` (全局上下文)

```
[Visibility]: Internal — musl mallocng 内部全局状态，不对外暴露
```

```c
extern struct malloc_context ctx;
```

其中 `malloc_context` 包含分配器全局状态，与 `malloc_usable_size` 相关的重要字段：
- `secret` (uint64_t): 随机密钥，用于验证 meta_area 的完整性。
- `active[48]`: 48 个大小类别各自的活跃 meta 链表头。

---

### `size_classes[]` (内部数组)

```
[Visibility]: Internal — musl mallocng 内部数组（虽有外部链接但标记为 hidden），不对外暴露
```

```c
extern const uint16_t size_classes[];
```

存储 48 个大小类别的 slot 大小（以 UNIT 为单位）。`size_classes[sc]` 给出大小类别 `sc` 对应的 slot 大小（单位为 UNIT，即需乘以 16 得到字节数）。

---

## 内部辅助函数（static inline）

### `get_slot_index`

```
[Visibility]: Internal — musl mallocng 内部 static inline 函数，不对外导出
```

```c
static inline int get_slot_index(const unsigned char *p);
```

**功能**：从指针 p 的带内元数据中提取 slot 索引。

**前置条件**：
- `p` 非 NULL，且是由 mallocng 分配的有效用户指针。

**后置条件**：
- 返回值 = `p[-3] & 31`，即指针前第 3 字节的低 5 位。
- 返回值处于 `[0, 31]` 之间（5 位无符号整数）。

**元数据布局**（关键设计）：
```
p[-4] : 非零偏移标记（若在非零偏移处 enframe）
p[-3] : 复合字节 —— 低 5 位 = slot 索引，高 3 位 = 保留大小编码
p[-2], p[-1] : uint16_t 存储从 group->storage 起始到 p 的偏移（单位为 UNIT）
```

**意图**：mallocng 将 slot 索引和保留大小信息内联存储在用户指针前 4 字节中（即"带内"元数据），避免为每个分配维护独立的元数据表。

---

### `get_meta`

```
[Visibility]: Internal — musl mallocng 内部 static inline 函数，不对外导出
```

```c
static inline struct meta *get_meta(const unsigned char *p);
```

**功能**：给定由 mallocng 分配的用户指针 p，返回其所属的 `struct meta` 控制块。

**前置条件**：
- `p` 非 NULL，且是由 mallocng （本模块）分配的有效用户指针。
- `(uintptr_t)p` 必须 16 字节对齐（`assert(!((uintptr_t)p & 15))`）。

**系统算法**（逐层解码）：

1. **读取偏移量**：从 `p[-2]` 读取 `uint16_t`，获得用户指针 p 在 group 内的偏移量（以 UNIT 为单位）。
2. **检测非零偏移 enframe**：若 `p[-4] != 0`，说明分配时采用了非零起始偏移（以增大地址重用间隔、辅助检测 double-free）。此时上述 16 位偏移为 0，实际偏移量存储在 `p[-8]` 的 `uint32_t` 中（值 > 0xFFFF）。
3. **逆推 group 基址**：`base = p - UNIT * offset - UNIT`。这里 `-UNIT` 是因为 group 头部恰好占 UNIT 字节。
4. **获取 meta 指针**：`meta = base->meta`。
5. **一致性校验**（通过 assert 保证）：
   - `meta->mem == base` —— meta 与 group 双向关联一致。
   - `index <= meta->last_idx` —— slot 索引不超出 group 范围。
   - slot 对应位在 `avail_mask` 和 `freed_mask` 中均为 0 —— 确认该 slot 确已分配。
   - `meta_area->check == ctx.secret` —— meta 所在页的完整性校验。
   - 若 `sizeclass < 48`：偏移量在 slot 边界内。
   - 若 `sizeclass == 63`（mmap 大块）：`sizeclass == 63`。
   - 若 `maplen > 0`：偏移量不超过分配范围。

**后置条件**：
- 返回值是指向有效 `struct meta` 的指针。
- 所有 assert 条件成立（若违反则程序崩溃）。

**意图**：mallocng 通过将偏移量嵌入分配指针前的几个字节，实现 O(1) 时间从任意用户指针回溯到其元数据，无需全局查找表。

---

### `get_stride`

```
[Visibility]: Internal — musl mallocng 内部 static inline 函数，不对外导出
```

```c
static inline size_t get_stride(const struct meta *g);
```

**功能**：返回给定 meta group 中每个 slot 的跨度（stride），即相邻 slot 起始地址的字节距离。

**前置条件**：
- `g` 是指向有效 `struct meta` 的指针。

**后置条件**：

- **Case 1**（mmap 大块：`g->last_idx == 0 && g->maplen > 0`）：
  `stride = g->maplen * 4096 - UNIT`
  即整个 mmap 区域减去 group 头部作为单个 slot 的大小。

- **Case 2**（常规大小类别）：
  `stride = UNIT * size_classes[g->sizeclass]`
  即查表获得该类别 slot 的标准字节大小。

**意图**：stride 统一了常规大小类别和 mmap 大块两种分配模式的 slot 大小计算，上层代码无需分支处理。

---

### `get_nominal_size`

```
[Visibility]: Internal — musl mallocng 内部 static inline 函数，不对外导出
```

```c
static inline size_t get_nominal_size(const unsigned char *p, const unsigned char *end);
```

**功能**：计算用户指针 p 对应的实际可用字节数。即 `malloc_usable_size` 的核心计算逻辑。

**前置条件**：
- `p` 和 `end` 均由调用者按 mallocng 的布局规则计算。
- `end = start + stride - IB`，即 slot 末尾减去 4 字节带内元数据。
- `p[-3]` 的高 3 位存储了"保留大小"编码（`reserved = p[-3] >> 5`）。

**系统算法**：

1. **读取保留大小编码**：`reserved = p[-3] >> 5`（值域 0-7）。

2. **解码保留大小**：
   - 若 `reserved < 5`：保留大小即为 `reserved` 字节。
   - 若 `reserved >= 5`（实际为 5）：保留大小溢出存储。此时 `end[-4]` 开始的 `uint32_t` 存储真实保留大小（值 >= 5），且 `end[-5]` 必须为 0（作为溢出标记检测字节）。

3. **计算可用大小**：`return end - reserved - p`。
   即 slot 的可用空间减去保留区域。

**后置条件**：
- 返回值 ∈ `[0, stride - IB]`。
- `assert(reserved <= end - p)` 确保保留大小不超出 slot 实际空间。
- `assert(!*(end - reserved))` 确保保留区域的第一个字节为零（标记字节）。

**意图**：mallocng 支持将同一个 slot 的部分空间保留不分配给用户（例如未来的 realloc 收缩或对齐需求），`reserved` 记录这部分保留字节数。用户真正可用的大小是 `slot 总空间 - IB - reserved`。当 reserved < 5 时直接内联在 `p[-3]` 的高 3 位；>= 5 时使用 slot 末尾的扩展存储。

---

## 对外导出函数

### `malloc_usable_size`

```
[Visibility]: Public — GNU 扩展 API，<malloc.h> 声明
```

```c
size_t malloc_usable_size(void *p);
```

**意图**：返回指针 p 所指向的内存块的实际可用大小（以字节为单位）。这个值可能大于原始 `malloc()` 请求的大小（因为分配器按大小类别向上取整），调用者可以利用这些额外的空间，但不应依赖它用于可移植代码。

**前置条件**：

- `p` 可以是以下之一：
  - 由 `malloc` / `calloc` / `realloc` 返回的有效指针（且未被 `free` 释放）。
  - NULL 指针。
- 不持有 `__malloc_lock`（此函数不会获取锁，调用者需确保在单线程或已加锁上下文中使用）。

**后置条件**：

- **Case 1**（`p == NULL`）：返回 0。这是 GNU 扩展的约定行为。

- **Case 2**（`p != NULL`，有效指针）：
  返回 p 所指向内存块的实际可用字节数。
  
  返回值 >= 原始请求大小（因为大小类别取整可能导致实际分配大于请求）。上界为当前 slot 的 stride - IB - 任何保留字节。
  
  计算过程：
  1. 通过 `get_meta(p)` 定位所属 meta 控制块。
  2. 通过 `get_slot_index(p)` 获取 slot 索引。
  3. 通过 `get_stride(g)` 获取 slot 跨度。
  4. 计算 slot 起始地址 `start = g->mem->storage + stride * idx`。
  5. 计算有效区域末尾 `end = start + stride - IB`。
  6. 通过 `get_nominal_size(p, end)` 从带内元数据解码可用大小。

**不变量**：

- 对于通过 `malloc(n)` 分配的指针 p，有 `malloc_usable_size(p) >= n`。
- 对于通过 `realloc(p, n)` 分配的指针 p，有 `malloc_usable_size(p) >= n`。
- 对于通过 `calloc(nmemb, size)` 分配的指针 p，有 `malloc_usable_size(p) >= nmemb * size`。

**系统算法**：

此函数不获取任何锁（`rdlock`/`wrlock`），仅执行带内元数据的纯读操作。这是因为：
- `get_meta` 中的 `assert` 检查依赖 `avail_mask` 和 `freed_mask`，这两个字段为 `volatile` 且通过原子操作（`a_cas`）修改，纯读访问无需加锁即可保证一致性。
- 指针 p 本身由调用者持有，只要未被并发 `free` 就不会出现 use-after-free。

**注意事项**：

- 此函数是 GNU 扩展，POSIX 标准未定义。可移植代码应避免依赖。
- 在多线程环境中，若指针 p 被另一个线程并发 `free` 或 `realloc`，行为未定义。
- `malloc_usable_size` 的返回值不能用于推断原始请求的大小——只能得知分配器实际预留的空间大小。