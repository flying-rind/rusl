# malloc_impl — Rust 接口归约

> 源 C spec: `src/malloc/oldmalloc/spec/malloc_impl.md`
> 对应 C 头文件: `src/malloc/oldmalloc/malloc_impl.h`
> 复杂度: Level 3（核心数据结构 + 常量 + 辅助函数定义）
> 本模块为 musl 旧版 malloc（oldmalloc）的基础设施模块，定义所有核心类型、常量、导航辅助函数。被 `malloc`、`aligned_alloc`、`malloc_usable_size` 模块共享使用。

---

## 依赖图

```
malloc_impl 模块 (pub(crate), 纯定义模块，无函数实现)
│
├── 外部依赖 (Rust core 库)
│   ├── core::mem::size_of                    → 替代 sizeof
│   ├── core::ffi::c_void                     → C void 指针等效
│   ├── core::sync::atomic::{AtomicI32, AtomicU64}
│   │                                          → 替代 C volatile int / volatile uint64_t
│   └── usize, u64, u32, u8                   → Rust 原语类型
│
├── 内部依赖 (rusl 其他模块, pub(crate))
│   ├── ./malloc::__bin_chunk                  → 声明于此，实现于 malloc 模块
│   │                                           (hidden 可见性, pub(crate) 跨模块)
│   └── (平台 syscall 层)                      → __madvise / __munmap 等
│                                                (由 malloc 模块内部使用)
│
├── 数据结构 (本模块定义, pub(crate))
│   ├── struct Chunk                           → 对应 C struct chunk
│   ├── struct Bin                             → 对应 C struct bin
│   └── struct MallocState                     → 对应 C 匿名 struct { binmap; bins[64]; split_merge_lock; }
│
├── 常量定义 (pub(crate))
│   ├── SIZE_ALIGN / SIZE_MASK / OVERHEAD      → 大小计算基础常量
│   ├── DONTCARE / RECLAIM                     → trim / madvise 阈值常量
│   ├── MMAP_THRESHOLD                         → 大块分配阈值
│   └── C_INUSE                                → chunk 占用标志位
│
├── Chunk 导航方法 (impl Chunk)
│   ├── chunk_size / chunk_psize               → 剥离标志位后的实际大小
│   ├── prev_chunk / next_chunk                → 物理相邻 chunk 遍历
│   └── chunk_to_mem                           → chunk 指针 → 用户指针
│
└── 独立辅助函数 (pub(crate))
    ├── mem_to_chunk                            → 用户指针 → chunk 指针
    ├── bin_to_chunk                            → bin 索引 → 哨兵 chunk 指针
    └── is_mmapped                              → 判断 chunk 是否 mmap 分配
```

---

## 数据结构

### `struct Chunk`

```rust
/// 堆块元数据结构。采用边界标记（boundary tag）设计，
/// 支持 O(1) 时间的相邻空闲块合并。
///
/// 必须使用 `#[repr(C)]` 以确保字段布局与 C 版本一致，
/// 因为 chunk 直接铺设于堆内存之上，chunk 指针与用户指针之间的
/// OVERHEAD 偏移计算依赖确切的字段偏移量。
#[repr(C)]
pub(crate) struct Chunk {
    /// 前一个物理相邻 chunk 的大小。
    /// 低 1 位 (bit 0) 复用为 `C_INUSE` 标志位：
    ///   - 置位 (1): 前一个 chunk 正在使用中，不可向后合并
    ///   - 清零 (0): 前一个 chunk 可能空闲，可尝试向后合并
    /// 对于 mmap 分配的 chunk，该字段存储从 chunk 起始到
    /// mmap 返回基地址的偏移量（extra 字段，不含 C_INUSE 标志）。
    pub(crate) psize: usize,

    /// 当前 chunk 的大小。
    /// 低 1 位 (bit 0) 语义取决于 chunk 状态：
    ///   - 正在使用中的常规堆 chunk: `csize & C_INUSE == 1`
    ///   - mmap 分配的 chunk:          `csize & C_INUSE == 0`（IS_MMAPPED 宏判断依据）
    ///   - 空闲 chunk (在 bin 中):     `csize & C_INUSE == 0`
    /// 实际大小通过 `csize & !C_INUSE`（即 `csize & -2`）获取。
    pub(crate) csize: usize,

    /// 空闲链表后继指针。
    /// 仅当 chunk 位于 bin 中时有效；已分配的 chunk 中，
    /// 该字段所在内存属于用户数据区（与用户数据重叠）。
    pub(crate) next: *mut Chunk,

    /// 空闲链表前驱指针。
    /// 仅当 chunk 位于 bin 中时有效；已分配的 chunk 中，
    /// 该字段所在内存属于用户数据区（与用户数据重叠）。
    pub(crate) prev: *mut Chunk,
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性，仅在 oldmalloc 子模块间共享。POSIX/C 标准未定义此类型。

**字段语义详表**:

| 字段 | 类型 | 语义 |
|------|------|------|
| `psize` | `usize` | 前一个物理相邻 chunk 的大小。低 1 位复用为 `C_INUSE` 标志：置位表示前一个 chunk 正在使用中（不可向后合并）；清零表示前一个 chunk 可能空闲。对于 mmap chunk，存储从 chunk 起始到 mmap 返回基地址的偏移量。 |
| `csize` | `usize` | 当前 chunk 的大小。低 1 位语义取决于 chunk 状态。实际大小通过 `csize & !C_INUSE` 获取。 |
| `next` | `*mut Chunk` | 空闲链表后继指针。仅 chunk 在 bin 中时有效。 |
| `prev` | `*mut Chunk` | 空闲链表前驱指针。仅 chunk 在 bin 中时有效。 |

**设计说明**:
- C 原版的 `struct chunk` 在 Rust 中重命名为 `Chunk`（Rust 命名约定：类型名使用 PascalCase）。
- `next`/`prev` 字段与用户数据区重叠：当 chunk 在用时，用户数据从 `chunk` 指针偏移 `OVERHEAD` 字节处开始，覆盖 `next`/`prev` 字段所在内存。`#[repr(C)]` 确保覆盖关系的字节偏移与 C 版一致。
- 所有字段保持 `usize` 类型，与原 C `size_t` 语义一致。

**前置条件**（创建/修改 chunk 时）:
- `csize` 和 `psize` 必须是 `SIZE_ALIGN` 的整数倍（除去低位的 `C_INUSE` 标志位）
- 对于常规堆 chunk：`csize` 的实际大小值（屏蔽 bit 0 后）必须 >= `OVERHEAD`
- `psize` 的值必须与实际前一个物理 chunk 的 `csize` 保持一致（一致性不变量）
- 对于 mmap chunk：`csize` 不含 `C_INUSE` 位，`psize` 存储对齐偏移量

**后置条件**:
- 修改 `csize` 后，必须同步更新后继物理 chunk 的 `psize` 以保持一致性

**不变量**:
- **双向链表完整性**: 若 chunk 在 bin 中，则 `(*c.next).prev == c` 且 `(*c.prev).next == c`
- **大小字段一致性**: 对于任意两个物理相邻的 chunk `a` 和 `b = a.next_chunk()`，有 `b.chunk_psize() == a.chunk_size()`
- **对齐不变量**: `chunk_size(c)` 总是 `SIZE_ALIGN` 的整数倍
- **LIFO 顺序**: 新释放的 chunk 插入 bin 头部

---

### `struct Bin`

```rust
/// 空闲链表桶结构。musl 使用 64 个 bin（`mal.bins[64]`），
/// 按 chunk 大小分桶，实现近似 best-fit 的分配策略。
///
/// 每个 bin 由自旋锁保护，包含一个哨兵 chunk 构成的双向循环链表。
///
/// # 哨兵设计
///
/// `head`/`tail` 字段与哨兵 chunk 的 `next`/`prev` 字段共用内存，
/// 实现零额外内存开销的哨兵节点。`bin_to_chunk(i)` 计算出的哨兵 chunk
/// 指针指向 `head` 字段之前 `OVERHEAD` 字节处，使得：
///   - `sentinel.next`  → `head`（指向链表中第一个实际 chunk）
///   - `sentinel.prev`  → `tail`（指向链表中最后一个实际 chunk）
///
/// 空链表时 `head == tail == bin_to_chunk(i)`。
#[repr(C)]
pub(crate) struct Bin {
    /// 自旋锁数组。
    /// - `lock[0]`: 锁值。0 = 未锁定，1 = 已锁定。通过原子 swap 获取。
    /// - `lock[1]`: 等待者计数。非零表示有线程在 futex 上等待该锁。
    /// 使用 `AtomicI32` 替代 C 的 `volatile int`，提供明确的原子语义。
    pub(crate) lock: [core::sync::atomic::AtomicI32; 2],

    /// 链表头指针。指向链表中第一个（最近释放的）chunk。
    /// 空链表时指向哨兵 chunk（即 `bin_to_chunk(i)`）。
    pub(crate) head: *mut Chunk,

    /// 链表尾指针。指向链表中最后一个（最早释放的）chunk。
    /// 空链表时指向哨兵 chunk。新 chunk 插入到 `tail` 之前。
    pub(crate) tail: *mut Chunk,
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性，仅在 oldmalloc 子模块间共享。POSIX/C 标准未定义此类型。

**字段语义详表**:

| 字段 | 类型 | 语义 |
|------|------|------|
| `lock[0]` | `AtomicI32` | 自旋锁的值。0 = 未锁定，1 = 已锁定。 |
| `lock[1]` | `AtomicI32` | 等待者计数。非零表示有线程在 `__wait` 上等待该锁。用于 futex 唤醒。 |
| `head` | `*mut Chunk` | 链表头指针。空链表时指向哨兵 chunk。 |
| `tail` | `*mut Chunk` | 链表尾指针。空链表时指向哨兵 chunk。 |

**设计说明**:
- C 原版的 `volatile int lock[2]` 在 Rust 中使用 `[AtomicI32; 2]`。`AtomicI32` 提供原子 compare_exchange / swap 操作，替代 C 中的 `a_swap` / `a_store` 宏。
- `AtomicI32` 在 `#[repr(C)]` 布局下与 `i32` 有相同的大小和对齐，因此 `[AtomicI32; 2]` 的布局与 C 的 `int lock[2]` 一致。
- `head` 和 `tail` 使用 `*mut Chunk` 裸指针类型，因为在哨兵设计中，这些字段的内存同时被解释为 chunk 的 `next`/`prev` 字段。

**前置条件**（访问 bin 时）:
- 访问 `head`/`tail` 或修改链表结构前，必须持有该 bin 的锁（`lock[0]` 为 1 且由当前线程持有）
- 首次使用某个 bin 前，必须通过 `lock_bin(i)` 初始化哨兵 chunk

**后置条件**:
- 释放锁后，链表结构保持一致（不变量成立）

**不变量**:
- **空链表表示**: `head == tail == bin_to_chunk(i)` 当且仅当链表为空
- **哨兵不变**: 哨兵 chunk 的 prev/next 指针构成自循环
- **binmap 一致性**: `MAL.binmap` 的 bit i 为 1 当且仅当 bin i 非空，该条件在持有锁时通过原子操作维护

---

### `struct MallocState`

```rust
/// 全局 malloc 状态结构体。整个 oldmalloc 分配器共享唯一一个实例 `MAL`。
///
/// 对应 C 代码中 `malloc.c` 的匿名 `static struct { ... } mal;`。
///
/// 该结构体在 `malloc_impl` 模块中定义，其具体静态实例
/// 定义于 `malloc` 模块（`static MAL: MallocState = ...`）。
#[repr(C)]
pub(crate) struct MallocState {
    /// 64 位位图，bit i 为 1 表示 bin i 非空。
    /// 通过原子 or/and 操作更新，支持 O(1) 时间找到第一个非空 bin。
    /// 使用 `AtomicU64` 替代 C 的 `volatile uint64_t`。
    pub(crate) binmap: core::sync::atomic::AtomicU64,

    /// 64 个 bin 数组。
    /// 索引 0~62 分别对应不同大小范围（按 chunk 大小分桶），
    /// 索引 63 包含所有超大 chunk（> MMAP_THRESHOLD 但仍来自堆的 chunk）。
    pub(crate) bins: [Bin; 64],

    /// 全局拆分/合并锁。
    /// 在 chunk 拆分（trim）和合并（__bin_chunk）操作期间持有，
    /// 防止并发修改导致 chunk 元数据不一致。
    /// - `split_merge_lock[0]`: 锁值（0 = 未锁定，1 = 已锁定）
    /// - `split_merge_lock[1]`: futex 等待者计数
    pub(crate) split_merge_lock: [core::sync::atomic::AtomicI32; 2],
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性，仅在 oldmalloc 子模块间共享。POSIX/C 标准未定义此类型。

**字段语义详表**:

| 字段 | 类型 | 语义 |
|------|------|------|
| `binmap` | `AtomicU64` | 64 位位图。bit i 为 1 表示 bin i 非空。通过 `a_or_64`/`a_and_64` 原子操作更新，支持 O(1) 时间找到第一个非空 bin（通过 `first_set` 函数）。 |
| `bins[64]` | `[Bin; 64]` | 64 个 bin 数组。bin 0~62 按大小分桶，bin 63 聚合所有超大 chunk。 |
| `split_merge_lock[0]` | `AtomicI32` | 全局拆分/合并锁的值。0 = 未锁定，1 = 已锁定。 |
| `split_merge_lock[1]` | `AtomicI32` | 全局拆分/合并锁的 futex 等待者计数。 |

**锁层次结构**:
```
MAL.split_merge_lock          ← 全局锁，最外层
    └── MAL.bins[i].lock      ← 各 bin 锁，内层
```

获取规则：先获取全局锁，再获取 bin 锁。释放时逆序。不允许在持有 bin 锁的情况下获取全局锁（防止死锁）。

**设计说明**:
- C 原版的匿名 `struct { volatile uint64_t binmap; struct bin bins[64]; volatile int split_merge_lock[2]; }` 在 Rust 中提升为具名结构体 `MallocState`。
- `binmap` 使用 `AtomicU64` 替代 `volatile uint64_t`，通过 `fetch_or`/`fetch_and` 提供显式原子语义。
- `split_merge_lock` 使用 `[AtomicI32; 2]` 替代 `volatile int[2]`，与 `Bin.lock` 设计一致。

---

## 常量定义

### SIZE_ALIGN / SIZE_MASK / OVERHEAD / DONTCARE / RECLAIM

```rust
/// chunk 大小的最小对齐单位。
/// 等于 `4 * sizeof(size_t)`。
/// - 64 位系统: 4 * 8 = 32 字节
/// - 32 位系统: 4 * 4 = 16 字节
/// 所有 chunk 的实际大小（`chunk_size(c)`）必须是该值的整数倍。
pub(crate) const SIZE_ALIGN: usize = 4 * core::mem::size_of::<usize>();

/// 对齐掩码。用于将任意值向下对齐到 `SIZE_ALIGN` 边界。
/// `x & SIZE_MASK` 等价于 C 的 `x & -SIZE_ALIGN`，清除低 `log2(SIZE_ALIGN)` 位。
/// 示例: SIZE_ALIGN=32 时, SIZE_MASK = 0xFFFFFFFFFFFFFFE0 (64位)
pub(crate) const SIZE_MASK: usize = SIZE_ALIGN.wrapping_neg();

/// 每个 chunk 的元数据开销。
/// 等于 `psize + csize` 两个 `size_t` 字段的大小。
/// 用户可用内存从 chunk 指针偏移 OVERHEAD 字节处开始。
/// - 64 位系统: 2 * 8 = 16 字节
/// - 32 位系统: 2 * 4 = 8 字节
///
/// 注意: 该值不含 `next`/`prev` 指针，因为它们仅在空闲 chunk 中存在，
/// 且与用户数据区重叠。
pub(crate) const OVERHEAD: usize = 2 * core::mem::size_of::<usize>();

/// 容忍浪费阈值。
/// 当请求大小与可用 chunk 大小的差值不超过该值时，
/// `trim` 操作跳过 chunk 拆分，直接将整个 chunk 分配给用户，
/// 避免产生过小碎片。
pub(crate) const DONTCARE: usize = 16;

/// 大块内存回收阈值（160 KB）。
/// 当 `__bin_chunk` 释放的 chunk 大小超过 `RECLAIM` 时，
/// 通过 `madvise(MADV_DONTNEED)` 将 chunk 中间部分
/// （去掉首尾对齐边界）的物理内存归还给操作系统。
pub(crate) const RECLAIM: usize = 163840;
```

**[Visibility]: Internal** — `pub(crate)` 可见性，仅在 oldmalloc 子模块间共享。POSIX/C 标准未定义这些常量。

**常量语义一览**:

| 常量 | 64位系统值 | 语义 |
|------|-----------|------|
| `SIZE_ALIGN` | 32 | chunk 大小的最小对齐单位 |
| `SIZE_MASK` | 0xFFFF...E0 | 向下对齐到 SIZE_ALIGN 的位掩码 |
| `OVERHEAD` | 16 | 单个 chunk 的元数据开销（psize + csize） |
| `DONTCARE` | 16 | trim 容忍浪费阈值，避免过小碎片 |
| `RECLAIM` | 163840 | 大块回收阈值，触发 madvise(MADV_DONTNEED) |

**不变量**:
- `SIZE_ALIGN` 必须是 2 的幂，使得 `SIZE_MASK` 可以正确工作
- `DONTCARE < OVERHEAD` 通常成立，确保不会因跳过 trim 而浪费超过元数据开销的空间

**设计说明**:
- C 原版的 `#define SIZE_MASK (-SIZE_ALIGN)` 在 Rust 中使用 `SIZE_ALIGN.wrapping_neg()`。对于 `usize` 类型，`wrapping_neg()` 执行二进制补码取负，与 C 的隐式 `size_t` 转换语义一致。
- C 原版的宏 `#define DONTCARE 16` 和 `#define RECLAIM 163840` 直接转换为 `const` 常量。
- 所有常量标记为 `pub(crate)`，在 oldmalloc 子模块间共享。

---

### MMAP_THRESHOLD

```rust
/// 大块内存分配阈值。
/// 当用户请求大小（经 `adjust_size` 处理后的 chunk 总大小）
/// 超过此阈值时，`malloc` 直接通过 `mmap` 系统调用分配独立内存映射，
/// 而非从堆空闲链表中分配。
///
/// 计算: MMAP_THRESHOLD = 0x1c00 * SIZE_ALIGN
/// - 64 位系统: 0x1c00 * 32 = 229376 字节 = 224 KB
/// - 32 位系统: 0x1c00 * 16 = 114688 字节 = 112 KB
///
/// 与 `bin_index` 的最大 bin 范围一致：
/// `bin_index(MMAP_THRESHOLD)` 返回 63，即最大 bin 索引。
pub(crate) const MMAP_THRESHOLD: usize = 0x1c00 * SIZE_ALIGN;
```

**[Visibility]: Internal** — `pub(crate)` 可见性。POSIX/C 标准未定义此常量。

**不变量**:
- `MMAP_THRESHOLD` 是 `SIZE_ALIGN` 的整数倍
- `MMAP_THRESHOLD <= bin` 索引 63 对应的最大大小

---

### C_INUSE

```rust
/// chunk 占用标志位。存储在 chunk 大小字段（`csize`/`psize`）
/// 的最低有效位 (bit 0)。
///
/// 由于所有 chunk 大小都是 `SIZE_ALIGN` 的倍数
/// （至少对齐到 16 或 32 字节），最低位天然为 0，
/// 可安全复用为状态标志。
///
/// ## 在 `csize` 上的语义:
///
/// | chunk 状态      | `csize & C_INUSE` | `is_mmapped(c)` | 含义                            |
/// |----------------|-------------------|-----------------|---------------------------------|
/// | 正在使用        | 1 (置位)           | false           | 常规堆 chunk                    |
/// | 正在使用 (mmap) | 0 (清零)           | true            | mmap 分配，psize 存储对齐偏移量  |
/// | 空闲 (在 bin 中) | 0                 | true (无意义)    | 调用者不应检查此宏               |
///
/// ## 在 `psize` 上的语义:
///
/// | `psize & C_INUSE` | 含义                                          |
/// |-------------------|-----------------------------------------------|
/// | 1                 | 前一个物理 chunk 正在使用中，不可向后合并       |
/// | 0                 | 前一个物理 chunk 可能空闲，可检查并尝试合并     |
pub(crate) const C_INUSE: usize = 1;
```

**[Visibility]: Internal** — `pub(crate)` 可见性。POSIX/C 标准未定义此常量。

---

## Chunk 导航方法（impl Chunk）

对应 C 中的 `CHUNK_SIZE(c)`、`CHUNK_PSIZE(c)`、`PREV_CHUNK(c)`、`NEXT_CHUNK(c)`、`CHUNK_TO_MEM(c)` 宏。在 Rust 中重新设计为 `Chunk` 的关联方法，利用 Rust 类型系统明确标注 `unsafe` 边界。

```rust
impl Chunk {
    /// 返回当前 chunk 的实际大小，剥离 `C_INUSE` 标志位。
    /// 等价于 C 宏 `CHUNK_SIZE(c)`: `(c)->csize & -2`
    ///
    /// 结果为 `SIZE_ALIGN` 的整数倍。
    #[inline]
    pub(crate) const fn chunk_size(&self) -> usize {
        self.csize & !C_INUSE
    }

    /// 返回前一个物理 chunk 的实际大小，剥离 `C_INUSE` 标志位。
    /// 等价于 C 宏 `CHUNK_PSIZE(c)`: `(c)->psize & -2`
    #[inline]
    pub(crate) const fn chunk_psize(&self) -> usize {
        self.psize & !C_INUSE
    }

    /// 返回指向前一个物理相邻 chunk 的指针。
    /// 等价于 C 宏 `PREV_CHUNK(c)`:
    /// `(struct chunk *)((char *)(c) - CHUNK_PSIZE(c))`
    ///
    /// # Safety
    /// - `self` 不能是堆的首 chunk（第一个 chunk 没有前驱）
    /// - `chunk_psize() > 0`
    #[inline]
    pub(crate) unsafe fn prev_chunk(&self) -> *mut Chunk {
        let base = self as *const Chunk as *const u8;
        base.sub(self.chunk_psize()) as *mut Chunk
    }

    /// 返回指向后一个物理相邻 chunk 的指针。
    /// 等价于 C 宏 `NEXT_CHUNK(c)`:
    /// `(struct chunk *)((char *)(c) + CHUNK_SIZE(c))`
    ///
    /// # Safety
    /// - `self` 不能是堆的末尾哨兵 chunk（其 `chunk_size()` 为 0）
    /// - `chunk_size() > 0`
    #[inline]
    pub(crate) unsafe fn next_chunk(&self) -> *mut Chunk {
        let base = self as *const Chunk as *const u8;
        base.add(self.chunk_size()) as *mut Chunk
    }

    /// 将 chunk 元数据指针转换为用户可见的内存指针。
    /// 等价于 C 宏 `CHUNK_TO_MEM(c)`: `(void *)((char *)(c) + OVERHEAD)`
    ///
    /// 返回值供 `malloc`/`realloc` 等函数返回给用户。
    #[inline]
    pub(crate) fn chunk_to_mem(&self) -> *mut core::ffi::c_void {
        unsafe {
            (self as *const Chunk as *const u8).add(OVERHEAD) as *mut core::ffi::c_void
        }
    }
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性，仅 oldmalloc 子模块可用。

**方法语义一览**:

| 方法 | 等价 C 宏 | 语义 |
|------|-----------|------|
| `chunk_size(&self) -> usize` | `CHUNK_SIZE(c)` | 返回 `csize & !C_INUSE`，即剥离标志位后的实际大小 |
| `chunk_psize(&self) -> usize` | `CHUNK_PSIZE(c)` | 返回 `psize & !C_INUSE`，前一个 chunk 的实际大小 |
| `prev_chunk(&self) -> *mut Chunk` | `PREV_CHUNK(c)` | 从当前 chunk 地址减去 `chunk_psize()` 得到前驱 chunk |
| `next_chunk(&self) -> *mut Chunk` | `NEXT_CHUNK(c)` | 从当前 chunk 地址加上 `chunk_size()` 得到后继 chunk |
| `chunk_to_mem(&self) -> *mut c_void` | `CHUNK_TO_MEM(c)` | 返回用户数据区指针（chunk + OVERHEAD） |

**设计说明**:
- `chunk_size` 和 `chunk_psize` 声明为 `const fn`，允许在编译期常量求值。
- `prev_chunk` 和 `next_chunk` 标记为 `unsafe`，因为它们依赖 chunk 大小字段的有效性和一致性不变量。调用者必须保证 chunk 指针有效且不是堆边界哨兵。
- `chunk_to_mem` 内部使用 `unsafe` 指针偏移，但从外部调用角度看，只要 chunk 指针有效，结果是安全的——返回的指针指向用户合法数据区。
- 使用 `#[inline]` 确保零成本抽象，编译后与原 C 宏内联展开等效。

**不变量**:
- `next_chunk(prev_chunk(c)) == c`（当 `c` 不是首 chunk 时）
- `prev_chunk(next_chunk(c)) == c`（当 `c` 不是尾哨兵时）
- `chunk_to_mem(mem_to_chunk(p)) == p`（往返恒等式）

---

## 独立辅助函数

对应 C 中的 `MEM_TO_CHUNK(p)`、`BIN_TO_CHUNK(i)`、`IS_MMAPPED(c)` 宏。在 Rust 中重新设计为独立的 `pub(crate)` 函数。

### `mem_to_chunk`

```rust
/// 将用户可见的内存指针转换为对应的 `Chunk` 元数据指针。
/// 等价于 C 宏 `MEM_TO_CHUNK(p)`:
/// `(struct chunk *)((char *)(p) - OVERHEAD)`
///
/// # Safety
/// - `p` 必须是由 `malloc`/`realloc`/`aligned_alloc` 等返回的有效用户指针，
///   或者由 `bin_to_chunk` 计算的哨兵地址
/// - 不应对栈变量、全局变量或已 `free` 的指针调用此函数
#[inline]
pub(crate) unsafe fn mem_to_chunk(p: *mut core::ffi::c_void) -> *mut Chunk {
    (p as *mut u8).sub(OVERHEAD) as *mut Chunk
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性。

**后置条件**:
- `chunk_to_mem(mem_to_chunk(p)) == p`（往返恒等式）

---

### `bin_to_chunk`

```rust
/// 计算第 `i` 个 bin 的哨兵 chunk 地址。
/// 等价于 C 宏 `BIN_TO_CHUNK(i)`:
/// `MEM_TO_CHUNK(&mal.bins[i].head)`
///
/// 哨兵 chunk 位于 `mal.bins[i].head` 字段内存位置之前 `OVERHEAD` 字节处，
/// 其 `next`/`prev` 指针覆盖 `head`/`tail` 字段，
/// 实现零额外内存开销的哨兵节点设计。
///
/// # 参数
/// - `i`: bin 索引，必须在 `[0, 63]` 范围内
/// - `mal`: 全局 `MallocState` 的不可变引用
///
/// # Safety
/// - `i` 必须在 `[0, 63]` 范围内
#[inline]
pub(crate) unsafe fn bin_to_chunk(i: usize, mal: &MallocState) -> *mut Chunk {
    let head_ptr: *const *mut Chunk = core::ptr::addr_of!(mal.bins[i].head);
    mem_to_chunk(head_ptr as *mut core::ffi::c_void)
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性。

**设计说明**:
- C 原版的 `BIN_TO_CHUNK(i)` 是宏，直接展开引用全局 `mal` 变量。在 Rust 中重新设计为显式接收 `&MallocState` 参数的函数，使依赖关系更清晰，也便于测试。
- 使用 `core::ptr::addr_of!` 安全地获取字段地址（不需要创建引用）。
- `head_ptr` 的类型为 `*const *mut Chunk`，强制转为 `*mut c_void` 后调用 `mem_to_chunk`。

---

### `is_mmapped`

```rust
/// 判断一个**正在使用中的** chunk 是否由 `mmap` 直接分配。
/// 等价于 C 宏 `IS_MMAPPED(c)`: `!((c)->csize & (C_INUSE))`
///
/// 对于 mmap chunk，`C_INUSE` 位恒为 0。
/// 该函数仅在 `free()` 和 `realloc()` 中用于区分 mmap chunk
/// （需要 `munmap`/`mremap`）和常规堆 chunk（需要 `__bin_chunk`）。
///
/// # 注意事项
/// - 空闲 chunk（在 bin 中）的 `csize & C_INUSE` 也为 0，
///   因此对空闲 chunk 调用此函数返回 true，但语义无意义。
///   调用者应确保只对使用中的 chunk 调用此函数。
#[inline]
pub(crate) fn is_mmapped(c: &Chunk) -> bool {
    c.csize & C_INUSE == 0
}
```

**[Visibility]: Internal** — `pub(crate)` 可见性。

**前置条件**:
- `c` 必须是有效的正在使用的 chunk 指针
- 调用者不应在 chunk 已被释放后调用此函数

**后置条件**:
- `is_mmapped(c) == true` => chunk 由 mmap 分配，释放时应调用 `unmap_chunk`（通过 `munmap`）
- `is_mmapped(c) == false` => chunk 是常规堆 chunk，释放时应调用 `__bin_chunk`

---

## 跨模块函数声明

### `__bin_chunk`

```rust
// 声明: 本模块不定义此函数，但所有 oldmalloc 子模块均依赖它。
// 实现位置: malloc 模块 (src/malloc/oldmalloc/malloc.rs)

/// 将 chunk 归还到空闲链表（bin）中，并与前后物理相邻的空闲 chunk
/// 尝试合并（coalescing），以减少外部碎片。
///
/// # Safety
/// - `self_` 必须指向一个当前正在使用中（C_INUSE 置位）的常规堆 chunk（非 mmap chunk）
/// - `self_.csize` 的 LSB 必须为 1（`C_INUSE` 置位）
/// - `next_chunk(self_).psize == self_.csize`（chunk 元数据一致性不变量）
/// - 调用者不持有任何 bin 锁，也不持有 `MAL.split_merge_lock`
///
/// # 后置条件
/// - chunk（可能已与相邻空闲块合并）被插入到对应大小 bin 的空闲链表中
/// - 合并后的 chunk 的 `csize` 和物理后继 chunk 的 `psize` 已更新
/// - 若合并后的大小 > RECLAIM，chunk 中间部分通过 `madvise(MADV_DONTNEED)` 归还 OS
/// - `errno` 的值在函数返回时被保留（不受内部 `madvise` 调用的影响）
/// - 函数返回时不持有任何锁
pub(crate) unsafe fn __bin_chunk(self_: *mut Chunk);
```

**[Visibility]: Internal (hidden)** — musl 内部函数，`pub(crate)` 可见性（与 C 中 `hidden` 语义等效），不对外部用户（libc 调用者）暴露。POSIX 和 C 标准均未定义此函数。

**设计说明**:
- C 原版的声明 `hidden void __bin_chunk(struct chunk *);` 在 `malloc_impl.h` 中声明，实现在 `malloc.c` 中。
- 在 Rust 中，函数不能先声明后定义（同一 crate 内），因此 `__bin_chunk` 将在 `malloc` 模块中定义，并通过 `pub(crate)` 导出给 `aligned_alloc` 等模块使用。
- 本模块（`malloc_impl`）仅在其 [RELY] 节中声明对此函数的依赖关系，表明：调用 `__bin_chunk` 的模块需要从 `malloc` 模块导入。

**不变量**:
- **合并保证**: 函数总是尽可能合并相邻空闲块（贪心合并策略）
- **无泄漏**: 每次调用必然将 chunk 插入某个 bin 中
- **errno 保持**: `madvise` 可能修改 `errno`，函数保证调用前后的 `errno` 值不变

---

## 关键不变量（跨函数全局属性）

1. **INV-CHUNK-SIZE**: chunk 的实际大小 `chunk_size(c)` 总是 `SIZE_ALIGN` 的整数倍。

2. **INV-PSIZE-CONSISTENCY**: 对于任意两个物理相邻的 chunk `a` 和 `b = a.next_chunk()`，恒有 `b.chunk_psize() == a.chunk_size()`。

3. **INV-LOCK-HIERARCHY**: 锁层次结构严格遵循 `split_merge_lock`（外）→ `bins[i].lock`（内）。不允许在持有 bin 锁的情况下获取全局锁。

4. **INV-BINMAP**: `MAL.binmap` 的 bit i 为 1 当且仅当 bin i 非空（`head != bin_to_chunk(i)`）。

5. **INV-BIN-LIST**: 若 chunk 在 bin 中，则 `(*c.next).prev == c` 且 `(*c.prev).next == c`。

6. **INV-MMAP-PSIZE**: 对于 mmap chunk，`psize` 存储从 chunk 结构体地址到 `mmap` 返回基址的偏移量（不含 `C_INUSE` 标志位）。

---

## 内存布局示意

```
全局 MallocState:
+-------------------+  <-- &MAL
| binmap (AtomicU64)|  8 bytes
+-------------------+
| split_merge_lock  |  8 bytes (2 * AtomicI32)
|   [0] [1]         |
+-------------------+
| bins[0]           |
|   lock[0] lock[1] |  8 bytes (2 * AtomicI32)
|   head            |  8 bytes (*mut Chunk)
|   tail            |  8 bytes (*mut Chunk)
+-------------------+
| bins[1]           |
|   ...             |
+-------------------+
| ... (64 bins)     |
+-------------------+

单个 Chunk (堆上):
+-------------------+
| psize: usize      |  8 bytes (64位)
| csize: usize      |  8 bytes (64位)
|  === OVERHEAD 边界 ===
| next: *mut Chunk  |  8 bytes (64位) ─┐ 用户数据区 (chunk 在用时)
| prev: *mut Chunk  |  8 bytes (64位) ─┘
+-------------------+

哨兵 Chunk (利用 Bin.head/tail):
+-------------------+  <-- bin_to_chunk(i)
| psize: (未使用)    |  8 bytes
| csize: (未使用)    |  8 bytes
| next: *mut Chunk  |  8 bytes = bin.head (零额外开销!)
| prev: *mut Chunk  |  8 bytes = bin.tail (零额外开销!)
+-------------------+

用户指针与 Chunk 的关系:
+-------------------+  <-- chunk 指针
| psize             |
| csize             |  <- OVERHEAD
| next (用户数据区)  |  <-- chunk_to_mem(c) = malloc 返回的指针
| prev (用户数据区)  |
|   ...user data... |
+-------------------+
```

---

## 跨模块依赖说明

| 依赖符号 | 定义位置 | 性质 |
|----------|----------|------|
| `MallocState` 静态实例 `MAL` | `malloc` 模块 (`src/malloc/oldmalloc/malloc.rs`) | `pub(crate)`，全局唯一实例 |
| `__bin_chunk` | `malloc` 模块 (`src/malloc/oldmalloc/malloc.rs`) | `pub(crate)` unsafe fn，hidden 等效 |
| `malloc` / `free` / `realloc` | `malloc` 模块 | C 标准 Public API，`#[no_mangle] pub extern "C"` |
| __mmap / __munmap / __madvise / __mremap | 平台 syscall 层 | `pub(crate)`，由 `malloc` 模块内部使用 |
| __wait / __wake | 平台同步原语层 | `pub(crate)`，futex 封装，由 `malloc` 模块内部使用 |
| `a_crash` 等效 | `core::intrinsics::abort()` 或自定义 | 由 `__bin_chunk` 内部使用 |

---

## [RELY]

```
Predefined Types (来自 Rust core 库):
  usize, u64, u32, u8                    -- Rust 基础原语类型
  core::ffi::c_void                      -- C void 类型等效
  core::mem::size_of                     -- 类型大小计算（替代 sizeof）
  core::sync::atomic::AtomicI32          -- 原子 i32，替代 C volatile int
  core::sync::atomic::AtomicU64          -- 原子 u64，替代 C volatile uint64_t
  core::ptr::addr_of!                    -- 安全获取字段地址（替代 &raw const）

Predefined Structures/Functions (来自 rusl malloc 模块, pub(crate)):
  // 定义于 malloc 模块 (src/malloc/oldmalloc/malloc.rs)
  //
  // MallocState 的全局唯一静态实例:
  //   pub(crate) static MAL: MallocState;
  //
  // bin_chunk 函数（将 chunk 插入 bin 链表）:
  //   unsafe fn bin_chunk(self_: *mut Chunk, i: usize);
  //   前置条件: 调用者持有 bin i 的锁
  //
  // unbin 函数（从 bin 链表中摘除 chunk）:
  //   unsafe fn unbin(c: *mut Chunk, i: usize);
  //   前置条件: 调用者持有 bin i 的锁
  //
  // lock / unlock 原语:
  //   unsafe fn lock(lk: *mut AtomicI32);
  //   unsafe fn unlock(lk: *mut AtomicI32);
  //
  // lock_bin / unlock_bin 原语:
  //   unsafe fn lock_bin(i: usize);
  //   unsafe fn unlock_bin(i: usize);
  //   首次 lock_bin(i) 时初始化哨兵 chunk
  //
  // __bin_chunk 函数（chunk 归还+合并）:
  //   pub(crate) unsafe fn __bin_chunk(self_: *mut Chunk);
  //   前置条件: 详见上文 __bin_chunk 节
  //
  // bin_index / bin_index_up 函数:
  //   fn bin_index(x: usize) -> usize;
  //   fn bin_index_up(x: usize) -> usize;
  //   大小到 bin 索引的映射
  //
  // first_set 函数:
  //   fn first_set(x: u64) -> usize;
  //   返回 u64 中最低置位的索引 (ctz)，用于 binmap 扫描

Platform Dependencies (平台 syscall 层, 由 malloc 模块内部使用):
  // 定义于 rusl 平台抽象层
  //   fn __madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int;
  //     -- madvise 系统调用的内部封装（用于 RECLAIM）
  //   fn __mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int,
  //             fd: c_int, off: off_t) -> *mut c_void;
  //     -- mmap 系统调用（用于 MMAP_THRESHOLD 大分配和 expand_heap）
  //   fn __munmap(addr: *mut c_void, len: usize) -> c_int;
  //     -- munmap 系统调用（用于释放 mmap chunk）
  //   fn __mremap(old: *mut c_void, old_len: usize, new_len: usize,
  //               flags: c_int) -> *mut c_void;
  //     -- mremap 系统调用（用于 realloc 中的 mmap chunk 扩展）
  //
  // 注意: rusl 禁止使用 libc crate，所有 syscall 必须通过
  // asm! 内联汇编或 core::arch 直接发起。

External Dependencies (标准 C 头文件, 已在 Rust 中内化):
  // sys/mman.h 常量:
  //   MADV_DONTNEED -- 用于 RECLAIM 时的 madvise 调用
  //   MAP_PRIVATE | MAP_ANONYMOUS -- mmap 分配标志
  //   PROT_READ | PROT_WRITE -- mmap 保护标志
  //   MAP_FAILED -- mmap 失败返回值
  //   以上常量由平台 syscall 层定义
  //
  // errno.h:
  //   EINVAL / ENOMEM -- 错误码，由 errno 模块提供
```

---

## [GUARANTEE]

```
Exported Interface (本模块, pub(crate) 可见性):

  // === 数据结构 ===
  //
  #[repr(C)]
  pub(crate) struct Chunk {
      pub(crate) psize: usize,
      pub(crate) csize: usize,
      pub(crate) next: *mut Chunk,
      pub(crate) prev: *mut Chunk,
  }
  // -- 堆块元数据结构，采用边界标记设计

  #[repr(C)]
  pub(crate) struct Bin {
      pub(crate) lock: [core::sync::atomic::AtomicI32; 2],
      pub(crate) head: *mut Chunk,
      pub(crate) tail: *mut Chunk,
  }
  // -- 空闲链表桶结构，含自旋锁和哨兵 chunk

  #[repr(C)]
  pub(crate) struct MallocState {
      pub(crate) binmap: core::sync::atomic::AtomicU64,
      pub(crate) bins: [Bin; 64],
      pub(crate) split_merge_lock: [core::sync::atomic::AtomicI32; 2],
  }
  // -- 全局 malloc 状态结构体

  // === 常量 ===
  //
  pub(crate) const SIZE_ALIGN: usize = 4 * core::mem::size_of::<usize>();
  pub(crate) const SIZE_MASK: usize = SIZE_ALIGN.wrapping_neg();
  pub(crate) const OVERHEAD: usize = 2 * core::mem::size_of::<usize>();
  pub(crate) const DONTCARE: usize = 16;
  pub(crate) const RECLAIM: usize = 163840;
  pub(crate) const MMAP_THRESHOLD: usize = 0x1c00 * SIZE_ALIGN;
  pub(crate) const C_INUSE: usize = 1;

  // === Chunk 导航方法 (impl Chunk) ===
  //
  impl Chunk {
      pub(crate) const fn chunk_size(&self) -> usize;
      pub(crate) const fn chunk_psize(&self) -> usize;
      pub(crate) unsafe fn prev_chunk(&self) -> *mut Chunk;
      pub(crate) unsafe fn next_chunk(&self) -> *mut Chunk;
      pub(crate) fn chunk_to_mem(&self) -> *mut core::ffi::c_void;
  }

  // === 独立辅助函数 ===
  //
  pub(crate) unsafe fn mem_to_chunk(p: *mut core::ffi::c_void) -> *mut Chunk;
  pub(crate) unsafe fn bin_to_chunk(i: usize, mal: &MallocState) -> *mut Chunk;
  pub(crate) fn is_mmapped(c: &Chunk) -> bool;

  // === 跨模块函数声明 (实现于 malloc 模块) ===
  //
  // 以下函数由 `malloc` 模块定义并提供，oldmalloc 子模块（如 aligned_alloc）
  // 通过 `use super::malloc::*` 导入:
  //
  //   pub(crate) unsafe fn __bin_chunk(self_: *mut Chunk);
  //     -- 将 chunk 归还到空闲链表并执行相邻空闲块合并
  //
  //   pub(crate) unsafe fn lock(lk: *mut AtomicI32);
  //   pub(crate) unsafe fn unlock(lk: *mut AtomicI32);
  //     -- 自旋锁原语
  //
  //   pub(crate) unsafe fn lock_bin(i: usize);
  //   pub(crate) unsafe fn unlock_bin(i: usize);
  //     -- bin 锁原语（首次调用时初始化哨兵 chunk）
  //
  //   pub(crate) unsafe fn unbin(c: *mut Chunk, i: usize);
  //     -- 从 bin 链表中摘除 chunk
  //
  //   pub(crate) unsafe fn bin_chunk(self_: *mut Chunk, i: usize);
  //     -- 将 chunk 插入 bin 链表
  //
  //   pub(crate) fn bin_index(x: usize) -> usize;
  //   pub(crate) fn bin_index_up(x: usize) -> usize;
  //     -- 大小到 bin 索引映射
  //
  //   pub(crate) fn first_set(x: u64) -> usize;
  //     -- u64 最低置位索引 (ctz)

  // === 全局状态 ===
  //
  // 定义于 malloc 模块:
  //   pub(crate) static MAL: MallocState;
  //     -- 全局唯一的 malloc 状态实例（包含 binmap, bins[64], split_merge_lock）

  // 注意: 本模块所有符号均为 pub(crate)，不对外部用户（libc 调用者）暴露。
  // 不存在 #[no_mangle] 或 pub extern "C" 的导出符号。
```

---

*本 Rust spec 通过递归依赖追踪生成。`malloc_impl` 是 musl oldmalloc 的基石模块——它定义了所有核心数据结构（Chunk, Bin, MallocState）、常量（SIZE_ALIGN, OVERHEAD, C_INUSE 等）和导航辅助函数（chunk_size, mem_to_chunk, is_mmapped 等），被 `malloc`、`aligned_alloc`、`malloc_usable_size` 三个模块共享使用。*

*递归依赖链: `malloc_impl` (本模块，定义层) → `malloc` (实现 `malloc`/`free`/`realloc`/`__bin_chunk` 及锁原语 `lock`/`unlock`/`lock_bin`/`unlock_bin`/`unbin`/`bin_chunk` 和辅助函数 `bin_index`/`first_set`) → `aligned_alloc` (对齐分配，调用 `malloc` + `__bin_chunk`) → `malloc_usable_size` (查询实际可用大小) → `replaced` 模块 (`__malloc_replaced`/`__aligned_alloc_replaced` 全局标志) → `dynlink` 动态链接器 (写入替换标志)。*

*C 原版使用 `#define` 宏实现 chunk 导航操作，通过头文件展开到各翻译单元。Rust 版本将宏重新设计为 `Chunk` 的关联方法和独立辅助函数，利用 Rust 类型系统和 `#[inline]` 标注确保零成本抽象——编译后与原 C 宏内联展开生成相同机器码，同时在源码层面提供类型安全和 `unsafe` 边界标注。*