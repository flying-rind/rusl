# meta.rs 规约 (Rust 版本)

> 本文件是 rusl mallocng (malloc new generation) 分配器的核心模块，定义了分配器所需的全部数据结构、常量和辅助函数。

---

## 依赖图

```
meta.rs (pub(crate) 模块)
├── 外部依赖 (Rust core 库)
│   ├── core::ffi::c_void         → void 指针类型等效
│   ├── core::sync::atomic::{AtomicI32, Ordering}
│   │                              → 替代 a_cas(), a_or() 原子操作
│   ├── u8, u16, u32, u64, usize  → Rust 原语类型
│   ├── core::mem::size_of        → 替代 sizeof
│   ├── core::ptr                  → 裸指针操作
│   └── core::intrinsics::abort()  → 替代 a_crash()
│
├── 内部依赖 (rusl 其他模块, pub(crate))
│   ├── crate::malloc::glue::{brk, mmap, madvise, mremap, munmap}
│   │                              → 系统调用封装 (see glue.rs spec)
│   ├── crate::malloc::context::{SIZE_CLASSES, CTX}
│   │                              → 大小类别表 & 全局上下文 (see context.rs spec)
│   └── crate::malloc::context::{alloc_meta, is_allzero}
│                                  → 元数据分配 & 全零检测 (see context.rs spec)
│
├── 数据结构 (本模块定义, pub(crate))
│   ├── struct Group                → 对应 C struct group
│   ├── struct Meta                 → 对应 C struct meta
│   ├── struct MetaArea             → 对应 C struct meta_area
│   └── struct MallocContext        → 对应 C struct malloc_context
│
├── 内联辅助函数 (pub(crate))
│   ├── queue / dequeue / dequeue_head  → 元数据链表操作
│   ├── free_meta                       → [依赖 queue]
│   ├── activate_group                  → [依赖 AtomicI32::compare_exchange]
│   ├── get_slot_index / get_meta       → 分配指针 → 元数据逆向解析
│   ├── get_nominal_size / set_size     → 分配块大小编解码
│   ├── get_stride                      → 组内槽位大小计算
│   ├── enframe                         → [依赖 get_stride, set_size]
│   ├── size_to_class / size_overflows  → [依赖 SIZE_CLASSES, usize::leading_zeros]
│   └── step_seq / record_seq / account_bounce / decay_bounces / is_bouncing
│                                       → 反碎片化序列号系统
│
└── 导出全局符号 (pub(crate), 跨 .rs 共享)
    ├── SIZE_CLASSES[]   → 定义于 context.rs
    ├── CTX              → 定义于 context.rs
    ├── alloc_meta()     → 定义于 context.rs
    └── is_allzero()     → 定义于 context.rs
```

---

## 常量定义

### MMAP_THRESHOLD

```rust
pub(crate) const MMAP_THRESHOLD: usize = 131052;
```

[Visibility]: Internal -- rusl mallocng 内部阈值，POSIX/C 标准未定义

**意图**: 当分配请求大小超过此阈值 (约 128KB) 时，分配器绕过 slab 机制，直接使用 `mmap` 独立分配，并独立 `munmap` 释放。

**设计说明**: C 原版使用 `#define` 宏，Rust 使用 `const` 常量。`usize` 类型确保与指针运算兼容。

---

### UNIT

```rust
pub(crate) const UNIT: usize = 16;
```

[Visibility]: Internal -- rusl mallocng 内部常量

**意图**: 最小分配对齐粒度。所有分配以 16 字节为步进单位。该值与 x86-64 ABI 对齐要求一致，同时作为 `struct Group` 的 header 偏移量。

---

### IB

```rust
pub(crate) const IB: usize = 4;
```

[Visibility]: Internal -- rusl mallocng 内部常量

**意图**: In-band header size，即每个槽位底部的"带内"元数据开销（4 字节，位于用户可用区域末尾之后）。每个 allocation slot 的实际存储空间为 `stride` 字节，其中 `IB` 字节用作越界检查标记。

---

### PGSZ

```rust
pub(crate) fn pgsz() -> usize {
    // 若编译时可确定，使用常量；否则从 CTX 获取运行时页大小
    #[cfg(PAGESIZE)]
    { PAGESIZE }
    #[cfg(not(PAGESIZE))]
    { unsafe { CTX.pagesize } }
}
```

[Visibility]: Internal -- rusl mallocng 内部常量/函数

**意图**: 页大小。若编译时可确定（`PAGESIZE` 已定义），则使用编译常量；否则在运行时从 `CTX.pagesize` 读取（由动态链接器或 `sysconf` 在初始化阶段填入）。

**设计说明**: C 原版使用条件编译的 `#define` 宏。Rust 使用 `#[cfg]` 条件编译的函数封装，确保调用方无需关心编译期/运行时差异。

---

## 数据结构

### struct Group

```rust
#[repr(C)]
pub(crate) struct Group {
    pub meta: *mut Meta,
    pub active_idx: u8,  // C: unsigned char active_idx:5
    pub pad: [u8; UNIT - core::mem::size_of::<*mut Meta>() - 1],
    // storage[] 柔性数组 -- 在 Rust 中表示为 DST (Dynamically Sized Type)
    // 实际使用时通过指针运算访问
}
```

[Visibility]: Internal -- rusl mallocng 内部分配组结构，POSIX/C 标准未定义

**意图**: 一组相同大小类别内存槽位的容器。是 slab 分配的基本单位。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `meta` | `*mut Meta` | 指向本组元数据的反向指针，用于从 `storage` 中的指针快速定位元数据 |
| `active_idx` | `u8` | 当前活动掩码的最高位编号 (0..31)，指示空闲槽位 `freed_mask` 中哪一位已被该组"认领" |
| `pad` | `[u8; N]` | 填充至 `UNIT` 字节对齐 |

**不变量**:
- `(*group.meta).mem == group as *mut Group as *mut u8` -- 元数据与组的双向绑定必须一致
- 整个 `Group` 起始地址按页对齐（由 `mmap` 保证）
- `storage` 区域中的每个槽位前 `IB` 字节为 in-band header，后 `IB` 字节为保留校验区

**设计说明**:
- C 原版的 `unsigned char active_idx:5` 位域在 Rust 中使用普通 `u8` 类型，位域约束由函数逻辑在运行时保证（`active_idx` 值域为 0..31）
- C 原版的 `char pad[UNIT - sizeof(struct meta *) - 1]` 在 Rust 中使用 `[u8; N]` 数组，`N` 通过 `const` 表达式计算
- C 原版的 `unsigned char storage[]` 柔性数组在 Rust 中不显式声明，而是通过裸指针偏移访问。实际使用中，`Group` 后紧邻的内存即是 `storage` 区域

---

### struct Meta

```rust
#[repr(C)]
pub(crate) struct Meta {
    pub prev: *mut Meta,
    pub next: *mut Meta,
    pub mem: *mut Group,
    pub avail_mask: AtomicI32,   // C: volatile int
    pub freed_mask: AtomicI32,   // C: volatile int
    // 位域合并为单个字段:
    //   last_idx:5, freeable:1, sizeclass:6, maplen:N
    // C: uintptr_t last_idx:5; uintptr_t freeable:1;
    //    uintptr_t sizeclass:6; uintptr_t maplen:8*sizeof(uintptr_t)-12;
    pub bitfields: usize,        // 位域复合字段
}
```

[Visibility]: Internal -- rusl mallocng 内部元数据结构，POSIX/C 标准未定义

**意图**: 描述一个 `Group` 的内存使用状态，同时充当链表节点存在于多种队列中（按 `sizeclass` 对应的 active 链表、free_meta 链表等）。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `prev` / `next` | `*mut Meta` | 双向循环链表指针，该 meta 在 active 链表或 free_meta 链表中的位置 |
| `mem` | `*mut Group` | 指向所描述的 `Group` |
| `avail_mask` | `AtomicI32` | 可用槽位位掩码，位 i 为 1 表示槽位 i 空闲可分配。原子类型替换 C 的 `volatile int` |
| `freed_mask` | `AtomicI32` | 释放槽位位掩码，位 i 为 1 表示槽位 i 已被释放但尚未被 `activate_group` 认领。原子类型替换 C 的 `volatile int` |
| `bitfields` | `usize` | 复合位域：`last_idx`(5位) + `freeable`(1位) + `sizeclass`(6位) + `maplen`(剩余位) |

**位域访问方法** (作为 `Meta` 的 impl 函数):

```rust
impl Meta {
    // last_idx: 低 5 位 [0..4]
    pub(crate) fn last_idx(&self) -> usize {
        self.bitfields & 0x1F
    }
    pub(crate) fn set_last_idx(&mut self, v: usize) {
        self.bitfields = (self.bitfields & !0x1F) | (v & 0x1F);
    }

    // freeable: 第 5 位
    pub(crate) fn freeable(&self) -> bool {
        (self.bitfields >> 5) & 1 != 0
    }
    pub(crate) fn set_freeable(&mut self, v: bool) {
        let bit = if v { 1usize << 5 } else { 0 };
        self.bitfields = (self.bitfields & !(1usize << 5)) | bit;
    }

    // sizeclass: 第 6-11 位 (6 位)
    pub(crate) fn sizeclass(&self) -> usize {
        (self.bitfields >> 6) & 0x3F
    }
    pub(crate) fn set_sizeclass(&mut self, v: usize) {
        self.bitfields = (self.bitfields & !(0x3F << 6)) | ((v & 0x3F) << 6);
    }

    // maplen: 第 12 位及以上 (剩余位)
    pub(crate) fn maplen(&self) -> usize {
        self.bitfields >> 12
    }
    pub(crate) fn set_maplen(&mut self, v: usize) {
        self.bitfields = (self.bitfields & 0xFFF) | (v << 12);
    }
}
```

**设计说明**:
- C 原版的 `volatile int` 字段在 Rust 中使用 `AtomicI32`，通过 `Ordering::Relaxed`/`Ordering::AcqRel` 控制内存顺序，提供与 C `volatile` 等效的语义但更安全
- C 原版的 4 个位域在 Rust 中合并为单个 `usize` 字段并通过访问器方法封装。这样避免了 Rust 位域的支持限制（Rust 不直接支持 C 风格的位域语法），同时保持相同的 ABI 布局
- 位域布局与 C 原版完全兼容（通过 `#[repr(C)]` + 手工位操作）
- `#[repr(C)]` 保证 `prev`, `next`, `mem` 指针与 `avail_mask`/`freed_mask`/`bitfields` 的对齐和偏移与 C 原版一致

**不变量**:
- 当路径经过 `get_meta()` 校验时，必须满足 `meta.mem == base` 且 `index <= meta.last_idx()`
- `avail_mask` 和 `freed_mask` 不相交（同一槽位不能同时处于可用和已释放状态）
- 若 `meta.prev.is_null() && meta.next.is_null()`，则该 meta 不在任何队列中
- 位域打包后 `size_of::<Meta>() <= 32`（通常为 4 个指针 + 2 个 AtomicI32 = 4*8 + 2*4 = 40；C 原版同样为此大小，因位域字段合并为一个 usize）

---

### struct MetaArea

```rust
#[repr(C)]
pub(crate) struct MetaArea {
    pub check: u64,
    pub next: *mut MetaArea,
    pub nslots: i32,
    // slots[] 柔性数组 -- 在 Rust 中通过指针偏移访问
}
```

[Visibility]: Internal -- rusl mallocng 内部结构，POSIX/C 标准未定义

**意图**: 按页对齐的内存区域，用于批量分配 `Meta`。每个 MetaArea 包含一个校验值、链表指针和若干 meta 槽位。该区域本身通过 `mmap` 分配，起始地址 4KB 对齐。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `check` | `u64` | 安全校验值，应等于 `CTX.secret`，用于防止伪造的指针攻击 |
| `next` | `*mut MetaArea` | 链表指针，链接所有 meta_area 实例 |
| `nslots` | `i32` | 槽位数量 |

**不变量**:
- `area.check == CTX.secret` -- 每次通过地址反查必须验证
- `(area as *const _ as usize) & 4095 == 0` -- 页对齐
- 有效 meta 的地址满足 `(meta as usize) & -4096 == area as usize`

**设计说明**:
- C 原版的 `struct meta slots[]` 柔性数组在 Rust 中不显式声明，而是通过指针偏移访问

---

### struct MallocContext

```rust
pub(crate) struct MallocContext {
    pub secret: u64,
    #[cfg(not(PAGESIZE))]
    pub pagesize: usize,
    pub init_done: i32,
    pub mmap_counter: u32,
    pub free_meta_head: *mut Meta,
    pub avail_meta: *mut Meta,
    pub avail_meta_count: usize,
    pub avail_meta_area_count: usize,
    pub meta_alloc_shift: usize,
    pub meta_area_head: *mut MetaArea,
    pub meta_area_tail: *mut MetaArea,
    pub avail_meta_areas: *mut u8,
    pub active: [*mut Meta; 48],
    pub usage_by_class: [usize; 48],
    pub unmap_seq: [u8; 32],
    pub bounces: [u8; 32],
    pub seq: u8,
    pub brk: usize,
}
```

[Visibility]: Internal -- rusl mallocng 全局分配上下文，POSIX/C 标准未定义

**意图**: 线程安全的全局分配器状态。整个 rusl mallocng 分配器共享唯一一个 `MallocContext` 实例 `CTX`。

**字段语义**:
| 字段 | 类型 | 含义 |
|------|------|------|
| `secret` | `u64` | 随机密钥，用于 MetaArea 校验和地址混淆，在 `malloc` 首次调用时初始化 |
| `pagesize` | `usize` | 运行时页大小（仅当编译时未定义 PAGESIZE 时存在） |
| `init_done` | `i32` | 初始化完成标志，0 表示未初始化 |
| `mmap_counter` | `u32` | mmap 调用计数器，用于触发周期性元数据回收 |
| `free_meta_head` | `*mut Meta` | 空闲 meta 双向循环链表头 |
| `avail_meta` | `*mut Meta` | 可用的 meta 区域起始指针 |
| `avail_meta_count` | `usize` | 可用 meta 计数 |
| `avail_meta_area_count` | `usize` | 可用 meta_area 计数 |
| `meta_alloc_shift` | `usize` | meta 区域分配的指数增长因子 |
| `meta_area_head` | `*mut MetaArea` | meta_area 链表头 |
| `meta_area_tail` | `*mut MetaArea` | meta_area 链表尾 |
| `avail_meta_areas` | `*mut u8` | 可用 meta_area 位图 |
| `active[48]` | `[*mut Meta; 48]` | 每个 sizeclass 的活跃 meta 双向循环链表头（48 个 size class） |
| `usage_by_class[48]` | `[usize; 48]` | 每个 sizeclass 的累计使用量 |
| `unmap_seq[32]` | `[u8; 32]` | 每个 size class (7-38) 最后一次 unmap 操作序列号 |
| `bounces[32]` | `[u8; 32]` | 每个 size class 的"弹跳"计数（频繁 map/unmap 的惩罚因子） |
| `seq` | `u8` | 全局操作序列计数器 (1-255)，每次分配/释放步进，用于检测 unmap 抖动 |
| `brk` | `usize` | 当前 brk 值（程序堆末端），用于扩展初始堆区域 |

**不变量**:
- `active[i]` 要么为 null（空链表），要么指向一个有效的双向循环链表头
- `free_meta_head` 要么为 null，要么指向有效双向循环链表头
- 全局 `CTX` 实例的访问必须在持有锁的情况下进行（多线程安全）

**设计说明**:
- C 原版使用 `struct malloc_context`，Rust 重命名为 `MallocContext`（遵循 Rust 命名约定，去掉 `struct` 前缀）
- C 原版的 `#ifndef PAGESIZE` 条件编译使用 `#[cfg(not(PAGESIZE))]` 替代
- C 原版的 `unsigned` 类型在 Rust 中明确为 `u32`
- C 原版的 `uintptr_t` 类型在 Rust 中使用 `usize`（语义等价）

---

## 辅助函数

### queue

```rust
/// 将 meta 节点插入双向循环链表尾部（效果上插入到头节点的前面）。
///
/// # Safety
/// - `phead` 非 null，指向链表头指针
/// - `m` 非 null，且当前不在任何链表中（`(*m).prev.is_null() && (*m).next.is_null()`）
/// - `*phead` 要么为 null，要么指向一个有效的循环链表
///
/// # Postcondition
/// - Case 链表原为空: `(*m).prev == m && (*m).next == m`，`*phead = m`
/// - Case 链表非空: `m` 被插入到 `*phead` 之前，循环链表完整性保持
pub(crate) unsafe fn queue(phead: *mut *mut Meta, m: *mut Meta);
```

[Visibility]: Internal -- rusl mallocng 内部链表操作函数，POSIX/C 标准未定义

**设计说明**: C 原版的 `static inline void queue(struct meta **phead, struct meta *m)` 转换为 Rust 的 `unsafe fn`，因为涉及裸指针操作。使用 `*mut *mut Meta` 表示"指向 Meta 指针的指针"。O(1) 循环链表尾部插入。

---

### dequeue

```rust
/// 从双向循环链表中移除 meta 节点。
///
/// # Safety
/// - `phead` 非 null
/// - `m` 非 null，且 `m` 必须在 `*phead` 指向的链表中
///
/// # Postcondition
/// - Case 链表只剩一个节点: `*phead = null`, `(*m).prev = (*m).next = null`
/// - Case 链表有多个节点: `m` 从链表中移除，前后节点正确重链
///   - 若 `*phead == m`，则 `*phead` 更新为 `(*m).next`
///   - `(*m).prev = (*m).next = null`
pub(crate) unsafe fn dequeue(phead: *mut *mut Meta, m: *mut Meta);
```

[Visibility]: Internal -- rusl mallocng 内部链表操作函数，POSIX/C 标准未定义

**设计说明**: O(1) 循环链表删除。

---

### dequeue_head

```rust
/// 从双向循环链表中取出并返回头节点。
///
/// # Safety
/// - `phead` 非 null
///
/// # Postcondition
/// - Case 链表为空: 返回 `null`
/// - Case 链表非空: 返回原 `*phead`，该节点已从链表中移除
pub(crate) unsafe fn dequeue_head(phead: *mut *mut Meta) -> *mut Meta;
```

[Visibility]: Internal -- rusl mallocng 内部链表操作函数，POSIX/C 标准未定义

**设计说明**: O(1)，委托给 `dequeue()`。

---

### free_meta

```rust
/// 将使用完毕的 meta 结构体清零并回收到全局 `CTX.free_meta_head` 空闲链表中。
///
/// # Safety
/// - `m` 非 null，指向一个不再使用的 `Meta`
/// - 调用者持有 malloc 锁
///
/// # Postcondition
/// - `m` 所有字段被清零
/// - `m` 被加入 `CTX.free_meta_head` 链表
///
/// 依赖: `queue()`
pub(crate) unsafe fn free_meta(m: *mut Meta);
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### activate_group

```rust
/// 通过原子 CAS 操作将 `freed_mask` 中 `active_idx` 范围内的已释放槽位转移到
/// `avail_mask` 中，使其变为可分配状态。
///
/// # Safety
/// - `m` 非 null
/// - 调用者持有 malloc 锁（至少 rdlock）
///
/// # Precondition
/// - `(*m).avail_mask` 的当前值为 0（组当前无可分配槽位，才会触发 activate）
///
/// # Postcondition
/// - `(*m).avail_mask` 包含原 `freed_mask` 中在 `active_idx` 位范围内的所有位
/// - `(*m).freed_mask` 中被认领的位已通过 CAS 原子清除
/// - 返回 `(*m).avail_mask` 的新值 (u32)
///
/// # Algorithm
/// - 计算公式 `act = (2u32 << (*m).mem.active_idx) - 1` 构造掩码
/// - 使用 `AtomicI32::compare_exchange` 原子 CAS 循环从 `freed_mask` 中取出低位释放槽位
pub(crate) unsafe fn activate_group(m: *mut Meta) -> u32;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

**设计说明**: C 原版使用 `a_cas` 宏。Rust 使用 `AtomicI32::compare_exchange`（通过 `Ordering::AcqRel` 确保原子性和内存顺序），提供等价的原子 CAS 语义。

---

### get_slot_index

```rust
/// 从分配指针的 in-band header 中提取槽位索引。
///
/// # Safety
/// - `p` 指向一个已分配块的起始地址
///
/// # Postcondition
/// - 返回 `(p.sub(3).read() & 31) as usize`，即 header 字节的低 5 位（0-31 的槽位索引）
pub(crate) unsafe fn get_slot_index(p: *const u8) -> usize;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

**设计说明**: C 原版的 `p[-3] & 31` 在 Rust 中使用 `p.sub(3).read() & 31`，更清晰地表达了指针偏移和读取操作。

---

### get_meta

```rust
/// 从任意分配指针逆向推导对应的 `Meta`。这是 rusl 设计中最核心的安全校验函数，
/// 通过多重断言确保指针合法性，防止 double-free、伪造指针等攻击。
///
/// # Safety
/// - `p` 非 null，`p as usize` 为 16 字节对齐
/// - `p` 指向一个由 mallocng 分配的合法内存块
///
/// # Postcondition
/// - Case 所有断言通过: 返回该块所属 group 的 `*mut Meta`
/// - Case 任一断言失败: `core::intrinsics::abort()` -- 进程立即终止（防内存损坏传播）
///
/// # 校验链 (按顺序):
/// 1. `debug_assert!((p as usize & 15) == 0)` -- 地址 16 字节对齐
/// 2. 读取 `p.sub(2)` 作为 16 位偏移量，`get_slot_index(p)` 获取槽位索引
/// 3. 若 `p.sub(4).read() != 0`，表明使用了非零起始偏移，则偏移量实际存储于 `p.sub(8)`，
///    且 `debug_assert!(offset > 0xFFFF)`
/// 4. 计算 group 基址 `base = p.sub(UNIT * offset + UNIT)`
/// 5. 通过 `(*base).meta` 获取元数据指针
/// 6. `debug_assert!((*meta).mem == base)` -- 双向绑定验证
/// 7. `debug_assert!(index <= (*meta).last_idx())` -- 索引不越界
/// 8. `debug_assert!(((*meta).avail_mask.load(Ordering::Relaxed) & (1u32 << index)) == 0)`
/// 9. `debug_assert!(((*meta).freed_mask.load(Ordering::Relaxed) & (1u32 << index)) == 0)`
/// 10. 计算 meta_area 指针（页对齐向下取整）
/// 11. `debug_assert!((*area).check == CTX.secret)` -- 密钥验证防伪造
/// 12. 对于 `sizeclass < 48`，验证偏移量与 sizeclass 的一致性
/// 13. 对于 `sizeclass == 63`（mmap 大对象），确认 `(*meta).sizeclass() == 63`
/// 14. 若 `(*meta).maplen()` 非零，验证偏移量不超过页映射范围
pub(crate) unsafe fn get_meta(p: *const u8) -> *mut Meta;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

**设计说明**:
- C 原版的 `assert()` 宏在 Rust 中使用 `debug_assert!()`（release 构建下移除）
- C 原版的 `a_crash()` 在 Rust 中使用 `core::intrinsics::abort()` 替代
- `Ordering::Relaxed` 用于读取 `avail_mask`/`freed_mask`，因为断言检查不需要同步顺序
- 此函数是 rusl 分配器安全性的基石，保留了原 C 实现的全部 14 步校验链

---

### get_nominal_size

```rust
/// 从分配块的 header 中恢复用户原始请求的分配大小（nominal size = 不含 reserved 区域的净大小）。
///
/// # Safety
/// - `p` 指向分配块起始地址
/// - `end` 指向分配块末尾地址（`p + stride - IB`）
/// - 分配的 header 格式合法
///
/// # Postcondition
/// - 返回 `end as usize - reserved - p as usize`，即用户可用字节数
///
/// # 编码解码规则:
/// - `reserved = p.sub(3).read() >> 5` 读取 reserved 值（高 3 位）
/// - 若 `reserved >= 5`，则实际 reserved 值存储在 `end.sub(4).cast::<u32>().read()`
///   且 `debug_assert!(reserved >= 5)`
/// - 大 reserved 情况额外校验 `debug_assert!(end.sub(5).read() == 0)`
/// - 校验 `debug_assert!(end.sub(reserved).read() == 0)`（分隔零字节）
/// - 校验 `debug_assert!(*end == 0)`（溢出检查字节）
pub(crate) unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### get_stride

```rust
/// 返回给定元数据所描述组中每个槽位的大小。
///
/// # Safety
/// - `g` 非 null
///
/// # Postcondition
/// - Case 独立 mmap (last_idx==0 && maplen>0): 返回 `maplen * 4096 - UNIT`
/// - Case 常规 slab 组: 返回 `UNIT * SIZE_CLASSES[(*g).sizeclass()]`
pub(crate) unsafe fn get_stride(g: *const Meta) -> usize;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### set_size

```rust
/// 在分配块的 in-band header 中写入用户请求大小 `n`（通过设置 reserved 区域来实现）。
///
/// # Safety
/// - `p` 指向分配块起始
/// - `end` 指向分配块末尾 `p + stride - IB`
/// - `n <= end as usize - p as usize`（请求大小不大于槽位容量）
///
/// # Postcondition:
/// - `reserved = end as usize - p as usize - n`
/// - 若 `reserved > 0`，则 `end.sub(reserved).write(0)`（设置分隔零字节）
/// - 若 `reserved >= 5`，则在 `end.sub(4).cast::<u32>().write(reserved as u32)`
///   并在 `end.sub(5).write(0)` 标记
/// - `p.sub(3)` 字节高 3 位被设置为 reserved（最大取 7，>=5 时取 5 用扩展编码）
pub(crate) unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize);
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### enframe

```rust
/// 在指定槽位中构造一个完整的新分配块。这是 `malloc()` 实际创建分配块的底层操作。
///
/// # Safety
/// - `g` 非 null，`(*g).mem` 非 null
/// - `idx` 是有效的槽位索引
/// - `n` 是用户请求的分配大小
/// - `ctr` 是分配计数器（用于随机化偏移）
///
/// # Postcondition
/// 返回用户可用指针 `*mut u8`，其 header 满足 `get_slot_index(p) == idx`。
/// 通过非零偏移和随机化递增，同一槽位连续分配时产生不同地址。
///
/// # 依赖:
/// - `get_stride(g)` -- 获取槽位总大小
/// - `set_size(p, end, n)` -- 写入大小信息
pub(crate) unsafe fn enframe(g: *mut Meta, idx: usize, n: usize, ctr: usize) -> *mut u8;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### size_to_class

```rust
/// 将用户请求大小 `n` 映射到大小类别索引 (0..47)。
///
/// # Postcondition
/// - 返回 0..47 的 sizeclass 索引，保证 `SIZE_CLASSES[sc] * UNIT >= n + IB - 1`
///
/// # Algorithm
/// 1. `n = (n + IB - 1) >> 4` -- 将字节大小转换为 UNIT 单位并向上取整
/// 2. 若 `n < 10` 直接返回 `n`
/// 3. 否则 `n += 1`，使用 `usize::leading_zeros()` 计算最高位位置
/// 4. 结合分段查表 `SIZE_CLASSES[]` 精确定位，通过两次比较修正索引
///
/// 依赖: `SIZE_CLASSES[]` (pub(crate) static, 定义于 context.rs)
pub(crate) fn size_to_class(n: usize) -> usize;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

**设计说明**: C 原版使用 `a_clz_32()` 宏。Rust 使用 `usize::leading_zeros()` 内建方法，语义等价且零成本（编译为 `lzcnt`/`clz` 指令）。

---

### size_overflows

```rust
/// 检查请求分配大小是否会导致溢出。
///
/// # Postcondition
/// - Case 溢出: `n >= usize::MAX / 2 - 4096`，返回 `true`
/// - Case 安全: 返回 `false`
///
/// # Note
/// - 调用者应负责设置 errno（由外层的 POSIX 适配层处理）
pub(crate) fn size_overflows(n: usize) -> bool;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

**设计说明**: C 原版直接设置 `errno = ENOMEM`。Rust 版本返回 `bool`，由调用者处理 errno 设置（因为 rusl 是 `#![no_std]`，errno 需通过平台抽象层处理）。

---

### step_seq

```rust
/// 推进全局操作序列计数器 `CTX.seq`。当计数器达上限 (255) 时，重置回 1 并清零所有 `unmap_seq[]`。
///
/// # Safety
/// - 调用者持有 malloc 锁
///
/// # Postcondition
/// - Case seq==255: 重置 seq=1，所有 unmap_seq[i]=0
/// - Case seq<255: seq++
pub(crate) unsafe fn step_seq();
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### record_seq

```rust
/// 记录某个 size class 最近一次触发 unmap 时的全局序列号。
///
/// # Postcondition
/// - 若 `sc >= 7 && sc < 39`，则 `CTX.unmap_seq[sc - 7] = CTX.seq`
pub(crate) unsafe fn record_seq(sc: usize);
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### account_bounce

```rust
/// 检测并记录特定 size class 的 map/unmap 抖动行为。
///
/// # Safety
/// - 调用者持有 malloc 锁
///
/// # Postcondition
/// - 若满足抖动条件（`sc >= 7 && sc < 39`，上次序列号非零，且
///   `CTX.seq - seq < 10`），则递增 `CTX.bounces[sc-7]`（上限 150）
pub(crate) unsafe fn account_bounce(sc: usize);
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### decay_bounces

```rust
/// 逐步衰减某个 size class 的弹跳计数。
///
/// # Postcondition
/// - 若 `sc >= 7 && sc < 39` 且 `CTX.bounces[sc-7] > 0`，则递减
pub(crate) unsafe fn decay_bounces(sc: usize);
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

### is_bouncing

```rust
/// 查询某个 size class 是否处于"弹跳"状态。
///
/// # Postcondition
/// - 若 `sc >= 7 && sc < 39` 且 `CTX.bounces[sc-7] >= 100`，返回 `true`
/// - 否则返回 `false`
pub(crate) unsafe fn is_bouncing(sc: usize) -> bool;
```

[Visibility]: Internal -- rusl mallocng 内部函数，POSIX/C 标准未定义

---

## 跨文件依赖全局符号

### SIZE_CLASSES[]

```rust
// 声明于 context.rs，通过 pub(crate) 导出
pub(crate) static SIZE_CLASSES: [u16; 48];
```

[Visibility]: Internal -- rusl mallocng 内部符号，仅在 mallocng 的 .rs 文件间共享。

**语义**: 大小类别查找表。`SIZE_CLASSES[i]` 表示第 i 个 size class 的槽位大小（以 UNIT 为单位）。

**设计说明**: C 原版的 `extern const uint16_t size_classes[]` + `__attribute__((__visibility__("hidden")))` 在 Rust 中改为 `pub(crate) static` 可见性。

---

### CTX

```rust
// 定义于 context.rs，通过 pub(crate) 导出
pub(crate) static mut CTX: MallocContext;
```

[Visibility]: Internal -- rusl mallocng 内部全局变量，仅在 mallocng 的 .rs 文件间共享。

**语义**: 全局唯一的 malloc 上下文实例。在 malloc 首次调用时初始化。

**多线程安全**: 对 `CTX` 的修改必须在持有 `__malloc_lock` 下进行。读取操作可能无需锁，但依赖 `AtomicI32` 保证一致性。

**设计说明**: C 原版的 `extern struct malloc_context ctx` 在 Rust 中重命名为 `CTX`（遵循 Rust 常量命名约定），类型重命名为 `MallocContext`。使用 `static mut` 声明（全局可变状态），由调用者确保加锁访问安全。

---

### alloc_meta()

```rust
// 定义于 context.rs，通过 pub(crate) 导出
pub(crate) fn alloc_meta() -> *mut Meta;
```

[Visibility]: Internal -- rusl mallocng 内部函数，仅在 mallocng 的 .rs 文件间共享。

**语义**: 分配一个新的 `Meta`，优先从 `CTX.free_meta_head` 空闲链表获取，否则通过 mmap 扩展 `MetaArea`。

**后置条件**: 返回一个已清零或从空闲链表中取出的 `*mut Meta`。失败时程序终止。

---

### is_allzero()

```rust
// 定义于 context.rs，通过 pub(crate) 导出
pub(crate) fn is_allzero(p: *mut c_void) -> i32;
```

[Visibility]: Internal -- rusl mallocng 内部函数，仅在 mallocng 的 .rs 文件间共享。

**语义**: 检查 `p` 指向的内存页是否全部为零。用于判断 madvise-free 后的页是否已被内核清零回收。

**后置条件**:
- 返回 1: 页面全为零
- 返回 0: 页面中有非零字节

**设计说明**: C 原版的 `void *` 参数在 Rust 中使用 `*mut c_void` 等效类型。

---

## 关键不变量 (跨函数全局属性)

1. **Header 自描述性**: 任何由 mallocng 返回的指针 `p` 必须满足：`p.sub(2)` 可解码为从 `p` 到 group 基址的偏移量，`p.sub(3).read() & 31` 可解码为槽位索引。这使得从任意指针反查元数据在 O(1) 内完成。

2. **INV-GET-META-01**: `get_meta(p)` 对任何合法分配指针必定成功返回且不与 `free()` 后的悬空指针产生误匹配。该不变量由多层 `debug_assert!` 保障。

3. **INV-MASK-01**: `avail_mask` 和 `freed_mask` 永不相交。即 `avail_mask & freed_mask == 0` 总是成立。

4. **INV-SLOT-COUNT-01**: 对于非 mmap 大对象组，槽位数 = `last_idx + 1`。

5. **INV-RESERVED-01**: 每个已分配块的 `end.sub(reserved)` 处有一个零字节作为分隔符，`*end` 处也有一个零字节作为溢出检测。

6. **INV-AREA-01**: 任意有效 `*mut Meta` 满足 `((meta as usize) & -4096 + offset_of!(MetaArea, check))` 处的 64 位值等于 `CTX.secret`。

7. **INV-SEQ-01**: `CTX.seq` 在 1..255 范围内循环。序列号回绕时，所有 `unmap_seq[]` 被清零。

8. **INV-BOUNCE-01**: 对任意 size class `sc`，若 `CTX.bounces[sc-7] >= 100`，则该 class 的 group 不应立即释放给内核。

---

## 内存布局示意

```
+------------------+  <-- group base (page-aligned)
| *mut Meta meta   |  8 bytes (on 64-bit)
| active_idx: u8    |  1 byte  (实际只用低 5 位)
| pad[N]            |  N bytes padding (填充至 UNIT)
+------------------+  <-- UNIT bytes from base
| storage[0]        |  slot 0 (stride bytes)
|  ...              |
|  [IB header]      |  in-band metadata (bottom IB bytes)
+------------------+
| storage[1]        |  slot 1 (stride bytes)
|  ...              |
+------------------+
|       ...         |
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

---

## [RELY]

Predefined Structures/Functions:
  // 来自 Rust core 库
  core::sync::atomic::AtomicI32     -- 原子 i32 类型，替代 C volatile int
  core::sync::atomic::Ordering       -- 内存顺序枚举
  core::ffi::c_void                  -- C void 类型等效
  core::mem::size_of                 -- 类型大小计算
  core::ptr                          -- 裸指针操作（read/write/sub/add）
  core::intrinsics::abort            -- 进程终止，替代 C a_crash()
  u8, u16, u32, u64, usize, i32     -- Rust 基础原语类型

  // 来自 rusl 内部模块
  crate::malloc::glue::{brk, mmap, madvise, mremap, munmap}
                                     -- 系统调用封装层 (see glue.rs spec)
  crate::malloc::context::SIZE_CLASSES
                                     -- 大小类别查找表: [u16; 48]
  crate::malloc::context::CTX        -- 全局分配器上下文: MallocContext
  crate::malloc::context::alloc_meta -- 元数据分配函数: fn() -> *mut Meta
  crate::malloc::context::is_allzero -- 全零检测: fn(*mut c_void) -> i32

  // 运行时环境依赖（由底层平台提供，非本 spec 范围）
  // - 页大小 (PAGESIZE / runtime)
  // - 动态链接器提供的 AT_RANDOM 随机种子
  // - pthread_atfork 锁机制（用于 fork 安全）

[GUARANTEE]
Exported Interface:
  // 本模块所有符号均为 Internal (pub(crate))，不对外部用户暴露
  //
  // 跨 .rs 文件共享的符号（由 context.rs 定义，meta.rs 引用）:
  //   pub(crate) static SIZE_CLASSES: [u16; 48];
  //   pub(crate) static mut CTX: MallocContext;
  //   pub(crate) fn alloc_meta() -> *mut Meta;
  //   pub(crate) fn is_allzero(p: *mut c_void) -> i32;
  //
  // meta.rs 本模块通过 pub(crate) 提供给其他 mallocng 模块的符号:
  //   - 常量: UNIT, IB, MMAP_THRESHOLD, pgsz()
  //   - 类型: Group, Meta, MetaArea, MallocContext
  //   - 核心函数: queue, dequeue, dequeue_head, free_meta, activate_group,
  //              get_slot_index, get_meta, get_nominal_size, get_stride,
  //              set_size, enframe, size_to_class, size_overflows,
  //              step_seq, record_seq, account_bounce, decay_bounces,
  //              is_bouncing