# malloc_impl.h 规约

## 模块概述

本头文件定义 musl 旧版 malloc 实现（oldmalloc）的核心数据结构、常量和辅助宏。这是 musl 内部的实现头文件，位于 `src/malloc/oldmalloc/`，不会被安装到系统 include 路径。它被 `malloc.c`、`aligned_alloc.c`、`malloc_usable_size.c` 包含，提供以下设施：

- 堆块（chunk）元数据结构 `struct chunk`
- 空闲链表桶（bin）数据结构 `struct bin`
- 大小对齐、边界计算、状态标志等宏
- 内部函数 `__bin_chunk` 的声明

---

## 依赖图

```
struct chunk  ─── (无依赖，自包含)
struct bin    ─── struct chunk
SIZE_ALIGN    ─── sizeof(size_t)
SIZE_MASK     ─── SIZE_ALIGN
OVERHEAD      ─── sizeof(size_t)
CHUNK_SIZE    ─── struct chunk (csize字段)
CHUNK_PSIZE   ─── struct chunk (psize字段)
PREV_CHUNK    ─── CHUNK_PSIZE, struct chunk
NEXT_CHUNK    ─── CHUNK_SIZE, struct chunk
MEM_TO_CHUNK  ─── OVERHEAD, struct chunk
CHUNK_TO_MEM  ─── OVERHEAD, struct chunk
BIN_TO_CHUNK  ─── MEM_TO_CHUNK, struct bin
C_INUSE       ─── (自包含常量)
IS_MMAPPED    ─── C_INUSE, struct chunk (csize字段)
MMAP_THRESHOLD── SIZE_ALIGN
__bin_chunk   ─── struct chunk (定义在 malloc.c)
```

---

## struct chunk (对外导出)

[Visibility]: **Public API** — 本头文件中声明，被 stdlib 内部实现使用。该结构体定义了 musl malloc 的堆块元数据格式。

### 函数签名
```c
struct chunk {
    size_t psize, csize;
    struct chunk *next, *prev;
};
```

### 意图
`struct chunk` 是 musl malloc 分配器中每一个内存块（已分配或空闲）的元数据头。它采用边界标记（boundary tag）设计：每个 chunk 的头部记录自身大小（`csize`）和前一个物理相邻 chunk 的大小（`psize`），从而支持 O(1) 时间的相邻空闲块合并。`next`/`prev` 指针构成双向链表，仅当 chunk 位于空闲桶（bin）中时有效。

### 字段语义

| 字段 | 类型 | 语义 |
|------|------|------|
| `psize` | `size_t` | 前一个物理相邻 chunk 的大小。低 1 位（bit 0）复用为 `C_INUSE` 标志：置位表示前一个 chunk 正在使用中（不可向后合并）；清零表示前一个 chunk 可能空闲（可尝试向后合并）。对于 mmap 分配的 chunk，该字段存储从 chunk 起始到 mmap 返回基地址的偏移量（不含 `C_INUSE` 标志）。 |
| `csize` | `size_t` | 当前 chunk 的大小。低 1 位（bit 0）的语义取决于 chunk 状态：对于**正在使用中的 chunk**，该位为 `C_INUSE` = 1 表示常规堆 chunk，为 0 表示 mmap 分配的 chunk（`IS_MMAPPED` 宏据此区分）。对于**空闲 chunk**（在 bin 中），该位总是 0。实际大小通过 `csize & -2` 获取。 |
| `next` | `struct chunk *` | 空闲链表后继指针。仅当 chunk 位于 bin 中时有效；已分配的 chunk 中该字段所在内存属于用户数据区。 |
| `prev` | `struct chunk *` | 空闲链表前驱指针。仅当 chunk 位于 bin 中时有效；已分配的 chunk 中该字段所在内存属于用户数据区。 |

### 前置条件（创建/修改 chunk 时）
- `csize` 和 `psize` 必须是 `SIZE_ALIGN` 的整数倍（除去低位的 `C_INUSE` 标志位）
- 对于常规堆 chunk：`csize` 的实际大小值（屏蔽 bit 0 后）必须 >= `OVERHEAD`
- `psize` 的值必须与实际前一个物理 chunk 的 `csize` 保持一致（一致性不变量）
- 对于 mmap chunk：`csize` 不含 `C_INUSE` 位，`psize` 存储对齐偏移量

### 后置条件
- 修改 `csize` 后，必须同步更新后继物理 chunk 的 `psize` 以保持一致性

### 不变量
- **双向链表完整性**：若 chunk 在 bin 中，则 `c->next->prev == c` 且 `c->prev->next == c`
- **大小字段一致性**：对于任意两个物理相邻的 chunk `a` 和 `b = NEXT_CHUNK(a)`，有 `CHUNK_PSIZE(b) == CHUNK_SIZE(a)`
- **对齐不变量**：`CHUNK_SIZE(c)` 总是 `SIZE_ALIGN` 的整数倍
- **LIFO 顺序**：新释放的 chunk 插入 bin 头部（通过 `bin_chunk` 实现）

### [RELY]
```
Predefined Types:
  size_t  — 定义于 <stddef.h>，无符号整数类型，用于表示内存大小
```

---

## struct bin (对外导出)

[Visibility]: **Public API** — 本头文件中声明，被 stdlib 内部实现使用。定义了 malloc 空闲链表的桶结构。

### 函数签名
```c
struct bin {
    volatile int lock[2];
    struct chunk *head;
    struct chunk *tail;
};
```

### 意图
`struct bin` 表示 malloc 分配器中的一个空闲链表桶（bin）。musl 使用 64 个 bin（`mal.bins[64]`），按 chunk 大小分桶，实现近似 best-fit 的分配策略。每个 bin 由自旋锁保护（`lock[0]` 为锁值，`lock[1]` 为等待计数），包含一个哨兵 chunk 构成的双向循环链表。

### 字段语义

| 字段 | 类型 | 语义 |
|------|------|------|
| `lock[0]` | `volatile int` | 自旋锁的值。0 = 未锁定，1 = 已锁定。通过 `a_swap` 原子操作获取。 |
| `lock[1]` | `volatile int` | 等待者计数。非零表示有线程在 `__wait` 上等待该锁。用于 futex 唤醒。 |
| `head` | `struct chunk *` | 链表头指针。指向链表中第一个（最近释放的）chunk。空链表时指向哨兵 chunk（即 `BIN_TO_CHUNK(i)`）。 |
| `tail` | `struct chunk *` | 链表尾指针。指向链表中最后一个（最早释放的）chunk。空链表时指向哨兵 chunk。新 chunk 插入到 `tail` 之前。 |

### 前置条件（访问 bin 时）
- 访问 `head`/`tail` 或修改链表结构前，必须持有该 bin 的锁（`lock[0] == 1` 且由当前线程持有）
- 首次使用某个 bin 前，必须通过 `lock_bin(i)` 初始化哨兵 chunk

### 后置条件
- 释放锁后，链表结构保持一致（不变量成立）

### 不变量
- **空链表表示**：`head == tail == BIN_TO_CHUNK(i)` 当且仅当链表为空
- **哨兵不变**：哨兵 chunk 的 prev/next 指针构成自循环
- **binmap 一致性**：`mal.binmap` 的 bit i 为 1 当且仅当 bin i 非空，该条件在持有锁时通过原子操作维护

### [RELY]
```
Predefined Structures:
  struct chunk  — 本头文件定义的堆块结构
Predefined Macros:
  BIN_TO_CHUNK  — 本头文件定义的宏，计算 bin 的哨兵 chunk 地址
```

---

## 大小与对齐常量宏 (对外导出)

[Visibility]: **Public API** — 本头文件中定义，被 malloc 内部实现使用。

### 函数签名
```c
#define SIZE_ALIGN (4*sizeof(size_t))
#define SIZE_MASK (-SIZE_ALIGN)
#define OVERHEAD (2*sizeof(size_t))
#define DONTCARE 16
#define RECLAIM 163840
```

### 意图与语义

| 宏 | 值（64位系统） | 语义 |
|----|---------------|------|
| `SIZE_ALIGN` | 32 | chunk 大小的最小对齐单位。所有 chunk 的实际大小（`CHUNK_SIZE`）必须是该值的整数倍。该值为 `4*sizeof(size_t)`，64 位系统上为 32，32 位系统上为 16。 |
| `SIZE_MASK` | -32 | 用于将任意值向下对齐到 `SIZE_ALIGN` 边界的位掩码。`x & SIZE_MASK` 等价于 `x & -SIZE_ALIGN`，清除低 `log2(SIZE_ALIGN)` 位。 |
| `OVERHEAD` | 16 | 每个 chunk 的元数据开销。等于 `psize` + `csize` 两个 `size_t` 字段的大小。用户可用的内存从 `OVERHEAD` 字节偏移处开始。实际开销可能更大（含对齐填充）。 |
| `DONTCARE` | 16 | 容忍浪费阈值。当请求大小与可用 chunk 大小的差值不超过该值时，不进行 chunk 拆分（trim 操作跳过），直接将整个 chunk 分配给用户，避免产生过小碎片。 |
| `RECLAIM` | 163840 | 大块内存回收阈值（160 KB）。当释放的 chunk 大小超过 `RECLAIM` 时，`__bin_chunk` 会通过 `madvise(MADV_DONTNEED)` 将 chunk 中间部分（去掉首尾对齐边界）的物理内存归还给操作系统。 |

### 不变量
- `SIZE_ALIGN` 必须是 2 的幂，使得 `SIZE_MASK` 可以正确工作
- `DONTCARE` < `OVERHEAD` 通常成立，确保不会因跳过 trim 而浪费超过元数据开销的空间

### [RELY]
```
Predefined Types:
  size_t  — 定义于 <stddef.h>
```

---

## MMAP_THRESHOLD 宏 (对外导出)

[Visibility]: **Public API** — 本头文件中定义，被 malloc 内部实现使用。

### 函数签名
```c
#define MMAP_THRESHOLD (0x1c00*SIZE_ALIGN)
```

### 意图
定义大块内存分配的阈值。当用户请求的大小超过 `MMAP_THRESHOLD` 时，`malloc` 直接通过 `mmap` 系统调用分配独立的内存映射，而非从堆空闲链表中分配。这避免了超大分配导致的内存碎片问题，且在 `free` 时可以直接 `munmap` 归还给操作系统。

### 语义
- **64 位系统**：`0x1c00 * 32 = 229376` 字节 = 224 KB
- **32 位系统**：`0x1c00 * 16 = 114688` 字节 = 112 KB
- 与 `bin_index` 的最大 bin 范围一致：`bin_index(MMAP_THRESHOLD)` 返回 63，即最大 bin 索引

### 不变量
- `MMAP_THRESHOLD` 是 `SIZE_ALIGN` 的整数倍
- `MMAP_THRESHOLD` <= bin 索引 63 对应的最大大小

### [RELY]
```
Predefined Macros:
  SIZE_ALIGN  — 本头文件定义的宏
```

---

## Chunk 导航与转换宏 (对外导出)

[Visibility]: **Public API** — 本头文件中定义，被 malloc 内部实现使用。

### 函数签名
```c
#define CHUNK_SIZE(c)  ((c)->csize & -2)
#define CHUNK_PSIZE(c) ((c)->psize & -2)
#define PREV_CHUNK(c)  ((struct chunk *)((char *)(c) - CHUNK_PSIZE(c)))
#define NEXT_CHUNK(c)  ((struct chunk *)((char *)(c) + CHUNK_SIZE(c)))
#define MEM_TO_CHUNK(p) (struct chunk *)((char *)(p) - OVERHEAD)
#define CHUNK_TO_MEM(c) (void *)((char *)(c) + OVERHEAD)
#define BIN_TO_CHUNK(i) (MEM_TO_CHUNK(&mal.bins[i].head))
```

### 意图
这组宏封装了 chunk 元数据与用户内存区域之间的双向转换，以及堆中物理相邻 chunk 的遍历，是 malloc 实现中最基础、最频繁使用的操作原语。

### 语义

| 宏 | 语义 |
|----|------|
| `CHUNK_SIZE(c)` | 返回 chunk `c` 的实际大小。清除 `csize` 的最低有效位（`-2` = `~1`），以剥离 `C_INUSE` 标志位。结果总是 `SIZE_ALIGN` 的倍数。 |
| `CHUNK_PSIZE(c)` | 返回前一个物理 chunk 的实际大小。清除 `psize` 的最低有效位。 |
| `PREV_CHUNK(c)` | 返回指向前一个物理相邻 chunk 的指针。通过从当前 chunk 地址减去前一个 chunk 的大小计算得到。 |
| `NEXT_CHUNK(c)` | 返回指向后一个物理相邻 chunk 的指针。通过从当前 chunk 地址加上当前 chunk 的大小计算得到。 |
| `MEM_TO_CHUNK(p)` | 将用户可见的内存指针 `p`（由 `malloc` 返回）转换为对应的 chunk 元数据指针。减去 `OVERHEAD` 字节偏移。 |
| `CHUNK_TO_MEM(c)` | 将 chunk 元数据指针 `c` 转换为用户可见的内存指针。加上 `OVERHEAD` 字节偏移。返回值供 `malloc`/`realloc` 等函数返回给用户。 |
| `BIN_TO_CHUNK(i)` | 计算第 `i` 个 bin 的哨兵 chunk 地址。哨兵 chunk 位于 `mal.bins[i].head` 字段内存位置之前 `OVERHEAD` 字节处，其 `next`/`prev` 指针覆盖 `head`/`tail` 字段，实现零额外内存开销的哨兵节点设计。 |

### 前置条件
- `CHUNK_SIZE(c)` / `CHUNK_PSIZE(c)`：chunk 指针 `c` 必须有效，且其 `csize`/`psize` 字段已正确初始化
- `PREV_CHUNK(c)`：`c` 不能是堆的起始 chunk（第一个 chunk 没有前驱），且 `CHUNK_PSIZE(c) > 0`
- `NEXT_CHUNK(c)`：`c` 不能是堆的末尾哨兵 chunk（其 `CHUNK_SIZE` 为 0），且 `CHUNK_SIZE(c) > 0`
- `MEM_TO_CHUNK(p)`：`p` 必须是由 `malloc`/`realloc` 等返回的有效用户指针，或者由 `BIN_TO_CHUNK` 计算的哨兵地址
- `CHUNK_TO_MEM(c)`：`c` 必须是有效的 chunk 指针
- `BIN_TO_CHUNK(i)`：`i` 在 `[0, 63]` 范围内；依赖全局静态变量 `mal`

### 后置条件
- 所有转换宏返回的指针指向有效的内存地址（假设输入有效）
- `CHUNK_TO_MEM(MEM_TO_CHUNK(p)) == p`（往返恒等式）
- `MEM_TO_CHUNK(CHUNK_TO_MEM(c)) == c`（往返恒等式）

### 不变量
- `NEXT_CHUNK(PREV_CHUNK(c)) == c`（当 `c` 不是首 chunk 时）
- `PREV_CHUNK(NEXT_CHUNK(c)) == c`（当 `c` 不是尾哨兵时）
- `CHUNK_SIZE(c) + CHUNK_SIZE(NEXT_CHUNK(c))` 不变（合并前后 chunk 总大小保持）

### [RELY]
```
Predefined Types:
  struct chunk  — 本头文件定义的堆块结构
  size_t        — 定义于 <stddef.h>
Predefined Macros:
  OVERHEAD      — 本头文件定义的元数据大小常量
  SIZE_ALIGN    — 本头文件定义的对齐常量
Predefined Variables:
  mal           — 全局 malloc 状态结构体，定义于 malloc.c，类型为匿名 struct { binmap; bins[64]; split_merge_lock; }
```

---

## C_INUSE 与 IS_MMAPPED 标志宏 (对外导出)

[Visibility]: **Public API** — 本头文件中定义，被 malloc 内部实现使用。

### 函数签名
```c
#define C_INUSE     ((size_t)1)
#define IS_MMAPPED(c) !((c)->csize & (C_INUSE))
```

### 意图
`C_INUSE` 是存储在 chunk 大小字段最低位（bit 0）的标志常量。由于所有 chunk 大小都是 `SIZE_ALIGN` 的倍数（至少对齐到 16 或 32），最低位天然为 0，可安全复用为状态标志。

`IS_MMAPPED` 通过检查 `csize` 的最低位判断一个**正在使用中的**chunk 是否由 `mmap` 直接分配（而非来自堆空闲链表）。该宏仅在 `free()` 和 `realloc()` 中用于区分 mmap chunk（需要 `munmap`/`mremap`）和常规堆 chunk（需要 `__bin_chunk`）。

### 语义

**`C_INUSE` 标志位在 `csize` 上的语义**：

| chunk 状态 | `csize & C_INUSE` | `IS_MMAPPED(c)` | 含义 |
|-----------|-------------------|-----------------|------|
| 正在使用 | 1（C_INUSE 置位） | false | 常规堆 chunk，由 `expand_heap` 或 bin 分配而来 |
| 正在使用 | 0（C_INUSE 清除） | true | mmap 分配的独立 chunk，`psize` 存储对齐偏移量 |
| 空闲（在 bin 中） | 0 | true（但无意义） | 调用者应只对使用中 chunk 调用此宏；`free()` 在调用 `__bin_chunk` 前已检查完毕 |

**`C_INUSE` 标志位在 `psize` 上的语义**：

| `psize & C_INUSE` | 含义 |
|-------------------|------|
| 1 | 前一个物理相邻 chunk 正在使用中，不可向后合并 |
| 0 | 前一个物理相邻 chunk 可能空闲，`__bin_chunk` 将检查并尝试合并 |

### 前置条件
- `IS_MMAPPED(c)`：`c` 必须是有效的正在使用的 chunk 指针
- 调用者不应在 chunk 已被释放后调用 `IS_MMAPPED`

### 后置条件
- `IS_MMAPPED(c) == true` → chunk 由 mmap 分配，释放时应调用 `unmap_chunk`
- `IS_MMAPPED(c) == false` → chunk 是常规堆 chunk，释放时应调用 `__bin_chunk`

### [RELY]
```
Predefined Types:
  size_t  — 定义于 <stddef.h>
```

---

## __bin_chunk 函数声明 (对外导出)

[Visibility]: **Internal (不导出)** — 该函数使用 `hidden` 可见性属性和 `__` 前缀命名约定，是 musl 内部分配器的核心实现函数。它不会出现在 libc.so 的导出符号表中，仅由 `malloc.c`（`free`、`realloc`、`__malloc_donate`）和 `aligned_alloc.c` 内部调用。POSIX 和 C 标准均未定义此函数。

### 函数签名
```c
hidden void __bin_chunk(struct chunk *self);
```

### 意图
将 chunk 归还到空闲链表（bin）中，并与前后物理相邻的空闲 chunk 尝试合并（coalescing），以减少外部碎片。函数定义在 `malloc.c:434`。

### 前置条件
- `self` 必须指向一个当前正在使用中（C_INUSE 置位）的常规堆 chunk（非 mmap chunk）
- `self->csize` 的 LSB 必须为 1（`C_INUSE` 置位）
- `NEXT_CHUNK(self)->psize == self->csize`（chunk 元数据一致性不变量）
- 调用者不持有任何 bin 锁，也不持有 `mal.split_merge_lock`

### 后置条件
- chunk（可能已与相邻空闲块合并）被插入到对应大小 bin 的空闲链表中
- 合并后的 chunk 的 `csize` 和 `NEXT_CHUNK(self)->psize` 已更新为新的合并大小
- 若合并后的大小 > `RECLAIM`，chunk 中间部分的内存通过 `madvise(MADV_DONTNEED)` 归还 OS
- `errno` 的值在函数返回时被保留（不受内部 `madvise` 调用的影响）
- 函数返回时不持有任何锁

### 算法（高层描述）
1. 验证 chunk 一致性（`NEXT_CHUNK(self)->psize == self->csize`），不一致则 crash
2. 获取全局 `split_merge_lock`
3. 检查前一个物理 chunk 是否空闲（`psize & C_INUSE == 0`），若是则获取其 bin 锁、从链表中摘除、合并大小
4. 检查后一个物理 chunk 是否空闲（`NEXT_CHUNK(self)->csize & C_INUSE == 0`），若是则同样合并
5. 计算合并后大小的 bin 索引，获取对应 bin 锁
6. 更新合并后的 chunk 元数据，插入 bin 链表
7. 释放 `split_merge_lock`
8. 若大小 > `RECLAIM`，通过 `madvise(MADV_DONTNEED)` 回收中间页
9. 释放 bin 锁

### 不变量
- **合并保证**：函数总是尽可能合并相邻空闲块（贪心合并策略）
- **无泄漏**：每次调用必然将 chunk 插入某个 bin 中（不会丢弃内存）
- **errno 保持**：`madvise` 可能修改 `errno`，函数保证调用前后的 `errno` 值不变

### [RELY]
```
Predefined Functions:
  a_crash       — 原子操作模块，用于检测到元数据损坏时主动崩溃
  lock / unlock — malloc.c 定义的锁原语
  lock_bin / unlock_bin — malloc.c 定义的 bin 锁原语
  unbin         — malloc.c 定义的从 bin 中摘除 chunk 的函数
  bin_chunk     — malloc.c 定义的将 chunk 插入 bin 的函数
  CHUNK_SIZE    — 本头文件定义的宏
  CHUNK_PSIZE   — 本头文件定义的宏
  PREV_CHUNK    — 本头文件定义的宏
  NEXT_CHUNK    — 本头文件定义的宏
  bin_index     — malloc.c 定义的大小到 bin 索引的映射函数
  __madvise     — mmap 系统调用的内部封装
  mal           — malloc.c 定义的全局状态变量

External Dependencies (skip):
  sys/mman.h    — 提供 MADV_DONTNEED 等常量
  errno.h       — 提供 errno 变量
```

---

## 全局状态依赖（跨文件说明）

`malloc_impl.h` 中的 `BIN_TO_CHUNK(i)` 宏引用了全局变量 `mal`，该变量定义于 `malloc.c`：

```c
static struct {
    volatile uint64_t binmap;
    struct bin bins[64];
    volatile int split_merge_lock[2];
} mal;
```

| 字段 | 语义 |
|------|------|
| `binmap` | 64 位位图，bit i 为 1 表示 bin i 非空。通过 `a_or_64`/`a_and_64` 原子操作更新，支持 O(1) 时间找到第一个非空 bin。 |
| `bins[64]` | 64 个 bin 数组，索引 0~62 分别对应不同大小范围，索引 63 包含所有超大 chunk（> MMAP_THRESHOLD 的常规堆 chunk）。 |
| `split_merge_lock[2]` | 全局拆分/合并锁。在 chunk 拆分（trim）和合并（__bin_chunk）操作期间持有，防止并发修改导致 chunk 元数据不一致。锁的设计与 bin 锁相同（`lock[0]` 为值，`lock[1]` 为等待计数）。 |

### 锁层次结构
```
mal.split_merge_lock           ← 全局锁，最外层
    └── mal.bins[i].lock       ← 各 bin 锁，内层
```

获取规则：先获取全局锁，再获取 bin 锁。释放时逆序。不允许在持有 bin 锁的情况下获取全局锁（防止死锁）。注意 `lock_bin`/`unlock_bin` 并不获取全局锁，因此调用 `bin_chunk`/`unbin` 时必须由调用者确保正确的锁层次。