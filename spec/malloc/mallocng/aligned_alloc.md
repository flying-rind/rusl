# aligned_alloc.c 规约

## 依赖图

```
aligned_alloc (对外导出)
  ├── malloc()                          — 外部 libc 函数
  ├── errno / EINVAL / ENOMEM           — 外部 libc 定义
  ├── SIZE_MAX                          — 外部 <limits.h>
  ├── DISABLE_ALIGNED_ALLOC             — 内部宏 (glue.h)
  │     └── __malloc_replaced           — 内部全局变量 (dynlink.h)
  │     └── __aligned_alloc_replaced    — 内部全局变量 (dynlink.h)
  ├── UNIT / IB                         — 内部常量 (meta.h)
  ├── struct meta                       — 内部结构体 (meta.h)
  ├── struct group                      — 内部结构体 (meta.h)
  ├── get_meta(p)                       — 内部 static inline (meta.h)
  ├── get_slot_index(p)                 — 内部 static inline (meta.h)
  ├── get_stride(g)                     — 内部 static inline (meta.h)
  └── set_size(p, end, n)              — 内部 static inline (meta.h)
```

> 注：`get_meta`、`get_slot_index`、`get_stride`、`set_size` 四个函数是定义在 `meta.h` 中的 `static inline` 函数，属于 mallocng 内部基础设施，其完整规约见 `meta.h` 的 spec。本文仅在 `aligned_alloc` 的调用上下文中描述其语义。

---

## 内部类型定义 (来自 meta.h，供 aligned_alloc 使用)

### struct meta

[Visibility]: Internal — musl mallocng 内部元数据结构，不对外导出

```c
struct meta {
    struct meta *prev, *next;          // 双向循环链表指针（用于组链表管理）
    struct group *mem;                 // 指向所属的 group
    volatile int avail_mask, freed_mask; // 可用/已释放槽位掩码
    uintptr_t last_idx:5;             // 组中最大槽位索引
    uintptr_t freeable:1;             // 组是否可被释放
    uintptr_t sizeclass:6;            // 大小类别索引 (0..47)，63 表示单独 mmap 的大块
    uintptr_t maplen:8*sizeof(uintptr_t)-12; // mmap 的页面数，0 表示非 mmap
};
```

`avail_mask` 和 `freed_mask` 是位掩码，每一位对应组内的一个槽位 (slot)。`avail_mask` 记录当前可直接使用的空闲槽位，`freed_mask` 记录已释放但可能尚未重新激活的槽位。

当 `sizeclass == 63` 时，表示这是一个通过 `mmap` 直接分配的单独大块（大于等于 `MMAP_THRESHOLD` 即 131052 字节）。

当 `maplen > 0` 时，表示该组由 `mmap` 单独分配，大小为 `maplen * 4096` 字节。`maplen == 0` 表示该组位于由其他组共享的大块内存中。

### struct group

[Visibility]: Internal — musl mallocng 内部内存组结构，不对外导出

```c
struct group {
    struct meta *meta;        // 指向此组的元数据
    unsigned char active_idx:5; // 当前已激活的最大槽位索引
    char pad[UNIT - sizeof(struct meta *) - 1]; // 填充至 UNIT 大小
    unsigned char storage[];  // 实际存储槽位的柔性数组
};
```

每个 `group` 包含一系列大小相等的槽位 (slots)，用于分配固定大小的内存块。`active_idx` 记录已激活（可使用的）槽位范围。`storage` 是柔性数组，实际存储槽位从此开始。`group` 结构体本身占用 `UNIT`(16) 字节，位于 `storage` 起始位置的前 `UNIT` 字节处。

### 头部字段布局（每个分配块的元数据）

每个由 mallocng 分配的指针 `p`，其前面有 4 个字节的元数据头部：

| 偏移 | 大小 | 含义 |
|------|------|------|
| `p[-4]` | 1 字节 | 偏移格式标志：`0` = 16-bit 偏移在 `p[-2]`，`非0` = 32-bit 偏移在 `p[-8]` |
| `p[-3]` | 1 字节 | 低 5 位：槽位索引 `idx`；高 3 位：保留大小 (reserved) |
| `p[-2]` | 2 字节 | 当 `p[-4]==0`：到 `group->storage` 的 16-bit 偏移（单位：UNIT）；当 `p[-4]!=0`：必须为 `0` |
| `p[-8:-5]` | 4 字节 | 当 `p[-4]!=0`：到 `group->storage` 的 32-bit 偏移（单位：UNIT） |

### 常量

[Visibility]: Internal — musl mallocng 内部常量，不对外导出

- `UNIT = 16`：基本对齐单位和记账单位。所有指针必须 16 字节对齐，group 头部占 16 字节。
- `IB = 4`：每个分配槽位末尾的内边界 (Internal Boundary) 大小。每个 slot 末尾的 IB 区域用于存储校验字节，防止越界破坏相邻槽位的头部。

---

## 内部 static inline 辅助函数 (来自 meta.h)

### get_meta

```c
static inline struct meta *get_meta(const unsigned char *p);
```

[Visibility]: Internal — musl mallocng 内部函数，不对外导出

**前置条件**：
- `p` 必须是由 mallocng 分配器返回的有效指针（16 字节对齐）
- `p` 的头部字段（`p[-4]`, `p[-3]`, `p[-2]` 或 `p[-8]`）必须是有效的、未损坏的

**后置条件**：
- **Case 1 (成功)**：返回指向 `p` 所属组的 `struct meta` 指针
- 内部执行多重完整性断言：
  - `p` 为 16 字节对齐
  - `meta->mem == base`（组指针一致）
  - `idx <= meta->last_idx`（槽位索引有效）
  - 目标槽位不在 `avail_mask` 或 `freed_mask` 中（即槽位当前被占用）
  - `meta_area->check == ctx.secret`（元数据区域校验通过）
  - 若 `meta->sizeclass < 48`：偏移量在对应大小类别的预期范围内
  - 若 `meta->maplen > 0`：偏移量不超过 `mmap` 范围

**算法简述**：
1. 从 `p[-2]` 读取 16-bit 偏移（若 `p[-4] != 0` 则从 `p[-8]` 读 32-bit 偏移）
2. 通过 `base = p - UNIT*offset - UNIT` 定位到 `struct group`（group 头部在 storage 前的 UNIT 字节）
3. 通过 `base->meta` 获取元数据指针，执行校验后返回

### get_slot_index

```c
static inline int get_slot_index(const unsigned char *p);
```

[Visibility]: Internal — musl mallocng 内部函数，不对外导出

**前置条件**：`p` 为有效分配指针

**后置条件**：返回 `p[-3] & 31`，即该分配在组内的槽位索引（0..31）

### get_stride

```c
static inline size_t get_stride(const struct meta *g);
```

[Visibility]: Internal — musl mallocng 内部函数，不对外导出

**前置条件**：`g` 为有效 `struct meta` 指针

**后置条件**：
- **Case 1 (g->last_idx == 0 && g->maplen > 0)**：返回 `g->maplen * 4096 - UNIT`（单独 mmap 的大块，stride 为整个映射大小减去 group 头部）
- **Case 2 (其他)**：返回 `UNIT * size_classes[g->sizeclass]`（常规大小类别的槽位跨度）

### set_size

```c
static inline void set_size(unsigned char *p, unsigned char *end, size_t n);
```

[Visibility]: Internal — musl mallocng 内部函数，不对外导出

**前置条件**：
- `p` 为有效分配指针（槽位起始位置）
- `end` 为该槽位的末尾地址（即 `storage + stride*(idx+1) - IB`）
- `n <= end - p`（请求大小不超过可用空间）

**后置条件**：
- 在 `end` 附近写入保留大小 `reserved = end - p - n`
- 若 `reserved > 0`，则设置 `end[-reserved] = 0` 作为边界标记字节
- 若 `reserved >= 5`，则需扩展存储：在 `end[-5]` 写入 `0`，在 `end[-4]` 写入 32-bit 的 `reserved` 值；然后将 `reserved` 截断为 5（头部 `p[-3]` 的高 3 位最多表示 7，即 reserved <= 7 可直接编码在头部，>=5 时用扩展格式）
- 更新 `p[-3]` 的高 3 位为 `reserved` 值

**意图**：记录分配块末尾的未用空间量，供 `free` 时恢复原始分配大小。

---

## aligned_alloc (对外导出)

```c
void *aligned_alloc(size_t align, size_t len);
```

[Visibility]: Public — POSIX 标准函数，`<stdlib.h>` 声明

### 前置条件

1. **对齐要求**：`align` 必须是 2 的幂（`(align & -align) == align`），否则调用失败
2. **大小要求**：`len` 必须是 `align` 的整数倍（POSIX 标准要求，本实现不做显式检查，但行为正确）
3. **溢出检查**：`len + align` 必须不超过 `SIZE_MAX`（`len <= SIZE_MAX - align`）
4. **对齐上限**：`align` 必须小于 `(1ULL << 31) * UNIT`（即小于 `2^31 * 16 = 32 GB`）
5. **分配器可用**：`DISABLE_ALIGNED_ALLOC` 必须为 `false`（即 `malloc` 未被替换 或 `aligned_alloc` 也一同被替换）

### 后置条件

**Case 1 (成功)**：返回一个至少 `len` 字节的已分配内存块指针 `p`，满足：
- `(uintptr_t)p % align == 0`（地址对齐到 `align` 边界）
- 内存块可安全写入 `len` 字节
- 分配块属于 mallocng 管理的某个 `struct group`，具有完整的元数据头部
- `p` 可通过标准 `free(p)` 安全释放

**Case 2 (失败)**：返回 `NULL` (`0`)，并设置 `errno`：
- `errno = EINVAL`：当 `align` 不是 2 的幂
- `errno = ENOMEM`：当 `len` 溢出、`align` 过大、`aligned_alloc` 被禁用、或底层 `malloc` 分配失败

### 系统算法 (Level 3)

`aligned_alloc` 的实现策略是 **过度分配 (over-allocate) 然后内部偏移 (internal offset)** ，而非请求 OS 直接提供对齐内存：

**阶段 1 — 参数校验**：
```
IF (align & -align) != align THEN errno=EINVAL, return 0
IF len > SIZE_MAX - align OR align >= (1ULL<<31)*UNIT THEN errno=ENOMEM, return 0
IF DISABLE_ALIGNED_ALLOC THEN errno=ENOMEM, return 0
IF align <= UNIT THEN align = UNIT  // 最小对齐为 16 字节
```

`(align & -align) != align` 是经典的 2 的幂判定：对 2 的幂 `n`，`n & -n == n`（补码性质）。
`DISABLE_ALIGNED_ALLOC` 定义为 `(__malloc_replaced && !__aligned_alloc_replaced)`：当用户替换了 `malloc` 但没有替换 `aligned_alloc` 时，禁止使用本实现的 `aligned_alloc`（因为本实现依赖 mallocng 内部的 `malloc`，而非用户替换的版本）。

**阶段 2 — 过度分配**：
```
p = malloc(len + align - UNIT)
```
分配比请求多 `align - UNIT` 字节的空间。由于 `malloc` 返回的指针已经是 16 字节对齐的，最坏情况下需要额外 `align - UNIT` 字节来保证能将指针提升到 `align` 对齐边界。

**阶段 3 — 获取槽位布局信息**：
```
g = get_meta(p)          // 获取元数据
idx = get_slot_index(p)  // 获取槽位索引
stride = get_stride(g)   // 获取槽位跨度
start = g->mem->storage + stride*idx      // 槽位起始地址
end   = g->mem->storage + stride*(idx+1) - IB  // 槽位末尾地址（减 IB 预留校验区）
adj   = -(uintptr_t)p & (align-1)         // 需要向上调整的字节数
```

`adj` 的计算：`-(uintptr_t)p & (align-1)` 等价于 `(align - (uintptr_t)p % align) % align`，即从当前地址到下一个 `align` 对齐边界所需的偏移量。利用了 2 的幂对齐的补码性质。

**阶段 4a — 已对齐的快速路径**：
```
IF adj == 0 THEN
    set_size(p, end, len)
    return p
```
若 `malloc` 返回的地址恰好在 `align` 边界上，无需任何调整，直接记录大小并返回。

**阶段 4b — 偏移调整并重写头部**：
```
p += adj                                    // 将指针偏移到对齐位置
offset = (p - g->mem->storage) / UNIT       // 计算新位置相对 storage 的偏移（单位：UNIT）
```

然后根据偏移量大小选择头部编码格式：

**小偏移（<= 0xffff）— 16-bit 编码**：
```
*(uint16_t *)(p-2) = offset     // 在 p[-2:-1] 存储 16-bit 偏移
p[-4] = 0                       // 标志：使用 16-bit 偏移
```

**大偏移（> 0xffff）— 32-bit 编码**：
```
*(uint16_t *)(p-2) = 0          // p[-2:-1] 必须为 0（配合 get_meta 的断言）
*(uint32_t *)(p-8) = offset     // 在 p[-8:-5] 存储 32-bit 偏移
p[-4] = 1                       // 标志：使用 32-bit 偏移
```

两种编码格式共享 `p[-4]` 作为鉴别标志：`0` 表示 16-bit 模式，`非0` 表示 32-bit 模式。

然后设置槽位索引和大小：
```
p[-3] = idx                     // 低 5 位记录槽位索引
set_size(p, end, len)           // 记录分配大小（可能覆盖 p[-3] 高 3 位）
```

**阶段 5 — 在原槽位头部写入"对齐 enframing"信息**：
```
*(uint16_t *)(start - 2) = (p - start) / UNIT   // 新位置相对原槽位起点的偏移
start[-3] = 7 << 5                               // 设置预留大小 = 7（最大值）
```

这一步在原 `malloc` 返回的地址 `start` 对应的头部写入信息：
- `start[-3] = 7<<5`：将 `reserved` 设为 7（最大值），表示从 `start` 开始的原始槽位有最大预留空间。这标记了该区域是"被 aligned_alloc 偏移过的"。
- `start[-2]`：记录 `p` 相对 `start` 的偏移，便于调试和堆遍历工具找到实际的对齐分配位置。

### 不变量

- **对齐不变量**：返回的指针 `p` 始终满足 `(uintptr_t)p % align == 0`（当 `align <= UNIT` 时，`align` 被提升为 `UNIT=16`）
- **元数据不变量**：返回的 `p` 必须能被 `get_meta(p)` 正确解析，即头部字段与组结构一致
- **槽位边界不变量**：`p + len <= end`，即用户可用空间不超过槽位的实际存储空间
- **offset 一致性**：`p[-2]`（或 `p[-8]` 的 32-bit）记录的偏移值乘以 `UNIT` 加上 `UNIT`（group 头部大小）必须能定位到 `group->storage`

### 复杂度

- **时间复杂度**：O(1) — 除 `malloc` 调用外，所有操作均为常数时间的指针运算和元数据读写
- **空间开销**：最多额外分配 `align - UNIT` 字节（用于对齐调整）。小对齐（如 32、64 字节）时开销极小；极端对齐（如 4KB）时开销接近一页

### 与 C11/POSIX 标准的关系

`aligned_alloc` 是 C11 标准引入的函数，POSIX.1-2017 采用。标准要求：
1. `align` 必须是 2 的幂
2. `len` 必须是 `align` 的整数倍
3. 返回的内存可通过 `free()` 释放

本实现满足上述所有要求。注意 C11 未定义 `align` 不是 2 的幂或 `len` 不是 `align` 整数倍时的行为；本实现中 `align` 非 2 的幂时返回 NULL 并设 `errno=EINVAL`，`len` 非整数倍时不报错但功能仍然正确（因为底层 `malloc` 分配了足够的空间）。

### 内部依赖符号汇总

| 符号 | 类型 | 来源 | 可见性 |
|------|------|------|--------|
| `aligned_alloc` | 函数 | aligned_alloc.c | **Public** — `<stdlib.h>` |
| `malloc` | 函数 | malloc.c (mallocng) | Public — `<stdlib.h>` |
| `errno` | 变量 | libc | Public — `<errno.h>` |
| `EINVAL` | 宏 | libc | Public — `<errno.h>` |
| `ENOMEM` | 宏 | libc | Public — `<errno.h>` |
| `SIZE_MAX` | 宏 | `<limits.h>` | Public — C 标准 |
| `UNIT` | 宏(16) | meta.h | Internal — mallocng 内部常量 |
| `IB` | 宏(4) | meta.h | Internal — mallocng 内部常量 |
| `DISABLE_ALIGNED_ALLOC` | 宏 | glue.h | Internal — mallocng 内部标志 |
| `struct meta` | 结构体 | meta.h | Internal — mallocng 内部类型 |
| `struct group` | 结构体 | meta.h | Internal — mallocng 内部类型 |
| `get_meta` | static inline 函数 | meta.h | Internal — mallocng 内部函数 |
| `get_slot_index` | static inline 函数 | meta.h | Internal — mallocng 内部函数 |
| `get_stride` | static inline 函数 | meta.h | Internal — mallocng 内部函数 |
| `set_size` | static inline 函数 | meta.h | Internal — mallocng 内部函数 |
| `size_classes` | extern 数组 | malloc.c | Internal — mallocng 内部数据 |
| `__malloc_replaced` | extern 变量 | replaced.c | Internal — musl 内部标志 |
| `__aligned_alloc_replaced` | extern 变量 | replaced.c | Internal — musl 内部标志 |

---

## 递归依赖终止说明

递归追踪在以下依赖处终止：

- `malloc()`：来自 libc，属于外部模块 — 其规约应在 `malloc.c` 的 spec 中独立描述
- `errno` / `EINVAL` / `ENOMEM`：C 标准库全局 errno 机制 — 外部模块
- `SIZE_MAX`：`<limits.h>` 定义的 C 标准宏 — 外部模块
- `get_meta` / `get_slot_index` / `get_stride` / `set_size`：`meta.h` 中的 `static inline` 函数 — 已在本文档中描述其语义，完整规约见 `meta.h` 的 spec
- `struct meta` / `struct group` / `UNIT` / `IB`：`meta.h` 中定义 — 已在本文档的类型定义和常量部分充分描述
- `DISABLE_ALIGNED_ALLOC`：`glue.h` 中的宏 — 已在本文档中解释其定义和用途
- `__malloc_replaced` / `__aligned_alloc_replaced`：来自 `replaced.c` / `dynlink.h` — 外部模块标志变量，用于判断分配器是否被替换