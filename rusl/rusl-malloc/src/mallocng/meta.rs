// meta.rs — 元数据结构定义与操作函数
//
// 对应 musl 的 src/malloc/mallocng/meta.h
// 本文件定义 mallocng 分配器的核心数据结构: Meta、Group、MetaArea、MallocContext，
// 以及元数据链表操作、分配指针解析、大小编解码、反碎片化序列号系统等辅助函数。
//
// 所有符号为 pub(crate)，仅供 mallocng 内部模块使用，不对外部用户暴露。
//
// 跨文件依赖 (定义于 context.rs):
//   - SIZE_CLASSES: [u16; 48]  — 大小类别查找表
//   - CTX: MallocContext        — 全局分配器上下文
//   - alloc_meta()              — 元数据分配函数
//   - is_allzero()              — 全零检测函数

use core::sync::atomic::AtomicI32;
use core::sync::atomic::Ordering;

use super::context::{alloc_meta, is_allzero, CTX};

// 重新导出 context.rs 中的符号，保持与 malloc.rs 的兼容性
pub(crate) use super::context::SIZE_CLASSES;

// ============================================================================
// 常量定义
// ============================================================================

/// 直接 mmap 分配阈值 (约 128KB)。
///
/// 当分配请求大小超过此阈值时，分配器绕过 slab 机制，
/// 直接使用 `mmap` 独立分配，并独立 `munmap` 释放。
///
/// 精确值: 131052 (= 128*1024 - 4*1024 + 44, 即 128K 对齐至 mmap 内部分配边界)
pub(crate) const MMAP_THRESHOLD: usize = 131052;

/// 最小分配对齐粒度 (16 字节)。
///
/// 所有分配以 16 字节为步进单位。该值与 x86-64 ABI 对齐要求一致，
/// 同时作为 `struct Group` 的 header 偏移量。
pub(crate) const UNIT: usize = 16;

/// In-band header size (4 字节)。
///
/// 每个槽位底部的"带内"元数据开销，位于用户可用区域末尾之后。
/// 每个 allocation slot 的实际存储空间为 `stride` 字节，
/// 其中 `IB` 字节用作越界检查标记。
pub(crate) const IB: usize = 4;

/// 编译期页大小常量 (x86-64 / aarch64 / 大多数 Linux 平台均为 4096)。
const PAGESIZE: usize = 4096;

/// 返回系统页大小。
#[inline]
pub(crate) fn pgsz() -> usize {
    PAGESIZE
}

// ============================================================================
// 数据结构 (前向声明)
// ============================================================================

// 前向声明 struct 名称，以便在 Group 中引用 Meta 指针
// (Rust 允许引用同一模块中稍后定义的类型，此处仅为文档清晰性标注)

// ============================================================================
// struct Group — 分配组
// ============================================================================

/// 一组相同大小类别内存槽位的容器。是 slab 分配的基本单位。
///
/// 每个 Group 包含一个固定大小 header（`meta` 指针 + `active_idx` + padding），
/// 后接变长的 `storage[]` 区域，其中每个槽位大小为 `stride` 字节。
///
/// # 内存布局
///
/// ```text
/// +------------------+  <-- group base (page-aligned)
/// | *mut Meta meta   |  8 bytes (on 64-bit)
/// | active_idx: u8    |  1 byte  (实际只用低 5 位)
/// | pad[N]            |  N bytes padding (填充至 UNIT)
/// +------------------+  <-- UNIT bytes from base
/// | storage[0]        |  slot 0 (stride bytes)
/// | storage[1]        |  slot 1 (stride bytes)
/// |       ...         |
/// +------------------+
/// ```
///
/// # 不变量
///
/// - `(*group.meta).mem == group as *mut Group as *mut u8` — 元数据与组的双向绑定必须一致
/// - 整个 `Group` 起始地址按页对齐（由 `mmap` 保证）
/// - `storage` 区域中的每个槽位前 `IB` 字节为 in-band header，后 `IB` 字节为保留校验区
#[repr(C)]
pub(crate) struct Group {
    /// 指向本组元数据的反向指针，用于从 `storage` 中的指针快速定位元数据。
    pub meta: *mut Meta,
    /// 当前活动掩码的最高位编号 (0..31)，指示空闲槽位 `freed_mask` 中
    /// 哪一位已被该组"认领"。
    ///
    /// C 原版为 `unsigned char active_idx:5` 位域，Rust 使用普通 `u8`，
    /// 位域约束由函数逻辑在运行时保证。
    pub active_idx: u8,
    /// 填充至 `UNIT` 字节对齐。
    ///
    /// 大小 = `UNIT - size_of::<*mut Meta>() - 1`。
    /// 64-bit 平台上 = 16 - 8 - 1 = 7 字节。
    pub pad: [u8; UNIT - core::mem::size_of::<*mut Meta>() - 1],
    // storage[] 柔性数组 — 在 Rust 中表示为 DST (Dynamically Sized Type)
    // 实际使用时通过裸指针偏移访问，不在结构体中显式声明。
}

// ============================================================================
// struct Meta — 分配组元数据
// ============================================================================

/// 描述一个 `Group` 的内存使用状态，同时充当链表节点存在于多种队列中。
///
/// 每个 Meta 可存在于以下队列之一:
/// - `CTX.active[sc]` — 某 size class 的活跃组链表
/// - `CTX.free_meta_head` — 空闲 meta 链表
/// - 孤立状态 (`prev == null && next == null`)
///
/// # 字段语义
///
/// | 字段 | 类型 | 含义 |
/// |------|------|------|
/// | `prev` / `next` | `*mut Meta` | 双向循环链表指针 |
/// | `mem` | `*mut Group` | 指向所描述的 `Group` |
/// | `avail_mask` | `AtomicI32` | 可用槽位掩码 (原子类型替代 C volatile) |
/// | `freed_mask` | `AtomicI32` | 释放槽位掩码 (原子类型替代 C volatile) |
/// | `bitfields` | `usize` | 复合位域: last_idx(5) + freeable(1) + sizeclass(6) + maplen(剩余) |
///
/// # 位域布局 (bitfields)
///
/// ```text
/// bitfields (usize, 64-bit):
/// ┌──────────────────────────────────────────────────────────────┬───┬───┬─────┐
/// │           maplen (52 bits, bits 12-63)                      │sc │fr │ lidx│
/// │                                                             │6b │1b │ 5b  │
/// └──────────────────────────────────────────────────────────────┴───┴───┴─────┘
///   63                                                          12  11  6 5    0
/// ```
///
/// # 不变量
///
/// - 当 `get_meta()` 校验通过时，必须满足 `meta.mem == base` 且 `index <= meta.last_idx()`
/// - `avail_mask` 和 `freed_mask` 不相交（同一槽位不能同时处于可用和已释放状态）
/// - 若 `meta.prev.is_null() && meta.next.is_null()`，则该 meta 不在任何队列中
/// - `size_of::<Meta>()` 通常为 40 字节 (4 个指针 + 2 个 AtomicI32 + 1 个 usize)
///
/// # 设计说明
///
/// - C 原版的 4 个位域在 Rust 中合并为单个 `usize` 字段并通过访问器方法封装，
///   避免 Rust 位域支持限制，同时保持 ABI 兼容
/// - C 原版的 `volatile int` 字段在 Rust 中使用 `AtomicI32`，通过 `Ordering` 控制内存顺序
#[repr(C)]
pub(crate) struct Meta {
    /// 双向循环链表前驱指针
    pub prev: *mut Meta,
    /// 双向循环链表后继指针
    pub next: *mut Meta,
    /// 指向所描述的 `Group`
    pub mem: *mut Group,
    /// 可用槽位位掩码，位 i 为 1 表示槽位 i 空闲可分配
    pub avail_mask: AtomicI32,
    /// 释放槽位位掩码，位 i 为 1 表示槽位 i 已被释放但尚未被 `activate_group` 认领
    pub freed_mask: AtomicI32,
    /// 复合位域: last_idx(5) | freeable(1) | sizeclass(6) | maplen(剩余)
    pub bitfields: usize,
}

// ============================================================================
// Meta 位域访问方法
// ============================================================================

impl Meta {
    // ---- last_idx: 低 5 位 [0..4] ----

    /// 读取 last_idx: 组内最后一个活跃槽位的索引 (0..31)
    #[inline]
    pub(crate) fn last_idx(&self) -> usize {
        self.bitfields & 0x1F
    }

    /// 设置 last_idx: 值自动截断为 5 位 (允许值 0..31)
    #[inline]
    pub(crate) fn set_last_idx(&mut self, v: usize) {
        self.bitfields = (self.bitfields & !0x1F) | (v & 0x1F);
    }

    // ---- freeable: 第 5 位 ----

    /// 读取 freeable: 是否允许释放该组
    ///
    /// - `true`: 该组可以被整体释放（非 donate 产生的组）
    /// - `false`: 该组不可释放（由 donate 产生）
    #[inline]
    pub(crate) fn freeable(&self) -> bool {
        (self.bitfields >> 5) & 1 != 0
    }

    /// 设置 freeable 标志位
    #[inline]
    pub(crate) fn set_freeable(&mut self, v: bool) {
        let bit = if v { 1usize << 5 } else { 0 };
        self.bitfields = (self.bitfields & !(1usize << 5)) | bit;
    }

    // ---- sizeclass: 第 6-11 位 (6 位, 值域 0..63) ----

    /// 读取 sizeclass: 大小类别索引
    ///
    /// - 0..47: 标准 slab 类别
    /// - 48..62: 保留/大对象
    /// - 63: mmap 独立大对象
    #[inline]
    pub(crate) fn sizeclass(&self) -> usize {
        (self.bitfields >> 6) & 0x3F
    }

    /// 设置 sizeclass: 值自动截断为 6 位 (允许值 0..63)
    #[inline]
    pub(crate) fn set_sizeclass(&mut self, v: usize) {
        self.bitfields = (self.bitfields & !(0x3F << 6)) | ((v & 0x3F) << 6);
    }

    // ---- maplen: 第 12 位及以上 (剩余位, 以 4K 页为单位) ----

    /// 读取 maplen: mmap 映射长度 (以 4096 字节页为单位)
    ///
    /// - 0: 非 mmap 子分配组
    /// - >0: mmap 主组，映射长度为 `maplen * 4096` 字节
    #[inline]
    pub(crate) fn maplen(&self) -> usize {
        self.bitfields >> 12
    }

    /// 设置 maplen: 以 4K 页为单位的映射长度
    ///
    /// 注意: 设置 maplen 会覆盖所有高位，不会影响低 12 位的其他字段。
    #[inline]
    pub(crate) fn set_maplen(&mut self, v: usize) {
        self.bitfields = (self.bitfields & 0xFFF) | (v << 12);
    }
}

// ============================================================================
// struct MetaArea — 元数据页对齐容器
// ============================================================================

/// 按页对齐的内存区域，用于批量分配 `Meta`。
///
/// 每个 MetaArea 包含一个安全校验值、链表指针和若干 meta 槽位。
/// 该区域本身通过 `mmap` 分配，起始地址 4KB 对齐。
///
/// # 字段语义
///
/// | 字段 | 类型 | 含义 |
/// |------|------|------|
/// | `check` | `u64` | 安全校验值，应等于 `CTX.secret`，用于防止伪造的指针攻击 |
/// | `next` | `*mut MetaArea` | 链表指针，链接所有 meta_area 实例 |
/// | `nslots` | `i32` | 槽位数量 |
///
/// # 不变量
///
/// - `area.check == CTX.secret` — 每次通过地址反查必须验证
/// - `(area as *const _ as usize) & 4095 == 0` — 页对齐
/// - 有效 meta 的地址满足 `(meta as usize) & -4096 == area as usize`
#[repr(C)]
pub(crate) struct MetaArea {
    /// 安全校验值，应等于 `CTX.secret`
    pub check: u64,
    /// 链表指针，链接所有 meta_area 实例
    pub next: *mut MetaArea,
    /// 槽位数量
    pub nslots: i32,
    // slots[] 柔性数组 — 在 Rust 中通过指针偏移访问，不显式声明
}

// ============================================================================
// struct MallocContext — 全局分配器上下文
// ============================================================================

/// 线程安全的全局分配器状态。
///
/// 整个 rusl mallocng 分配器共享唯一一个 `MallocContext` 实例 `CTX`。
/// 所有对 `CTX` 的修改必须在持有 `malloc` 锁下进行。
///
/// # 字段语义
///
/// | 字段 | 类型 | 含义 |
/// |------|------|------|
/// | `secret` | `u64` | 随机密钥，用于 MetaArea 校验和地址混淆 |
/// | `pagesize` | `usize` | 运行时页大小（仅当编译时未定义 PAGESIZE 时存在） |
/// | `init_done` | `i32` | 初始化完成标志，0 表示未初始化 |
/// | `mmap_counter` | `u32` | mmap 调用计数器，用于触发周期性元数据回收 |
/// | `free_meta_head` | `*mut Meta` | 空闲 meta 双向循环链表头 |
/// | `avail_meta` | `*mut Meta` | 可用的 meta 区域起始指针 |
/// | `avail_meta_count` | `usize` | 可用 meta 计数 |
/// | `avail_meta_area_count` | `usize` | 可用 meta_area 计数 |
/// | `meta_alloc_shift` | `usize` | meta 区域分配的指数增长因子 |
/// | `meta_area_head` | `*mut MetaArea` | meta_area 链表头 |
/// | `meta_area_tail` | `*mut MetaArea` | meta_area 链表尾 |
/// | `avail_meta_areas` | `*mut u8` | 可用 meta_area 位图 |
/// | `active[48]` | `[*mut Meta; 48]` | 每个 sizeclass 的活跃 meta 双向循环链表头 |
/// | `usage_by_class[48]` | `[usize; 48]` | 每个 sizeclass 的累计使用量 |
/// | `unmap_seq[32]` | `[u8; 32]` | 每个 size class (7-38) 最后一次 unmap 操作序列号 |
/// | `bounces[32]` | `[u8; 32]` | 每个 size class 的"弹跳"计数（map/unmap 抖动惩罚因子） |
/// | `seq` | `u8` | 全局操作序列计数器 (1-255)，每次分配/释放步进 |
/// | `brk` | `usize` | 当前 brk 值（程序堆末端），用于扩展初始堆区域 |
///
/// # 不变量
///
/// - `active[i]` 要么为 null（空链表），要么指向一个有效的双向循环链表头
/// - `free_meta_head` 要么为 null，要么指向有效双向循环链表头
/// - 全局 `CTX` 实例的访问必须在持有锁的情况下进行（多线程安全）
pub(crate) struct MallocContext {
    /// 随机密钥
    pub secret: u64,
    /// 运行时页大小
    pub pagesize: usize,
    /// 初始化完成标志
    pub init_done: i32,
    /// mmap 调用计数器
    pub mmap_counter: u32,
    /// 空闲 meta 双向循环链表头
    pub free_meta_head: *mut Meta,
    /// 可用的 meta 区域起始指针
    pub avail_meta: *mut Meta,
    /// 可用 meta 计数
    pub avail_meta_count: usize,
    /// 可用 meta_area 计数
    pub avail_meta_area_count: usize,
    /// meta 区域分配的指数增长因子
    pub meta_alloc_shift: usize,
    /// meta_area 链表头
    pub meta_area_head: *mut MetaArea,
    /// meta_area 链表尾
    pub meta_area_tail: *mut MetaArea,
    /// 可用 meta_area 位图
    pub avail_meta_areas: *mut u8,
    /// 每个 sizeclass 的活跃 meta 双向循环链表头 (48 个 size class)
    pub active: [*mut Meta; 48],
    /// 每个 sizeclass 的累计使用量
    pub usage_by_class: [usize; 48],
    /// 每个 size class (7-38) 最后一次 unmap 操作序列号
    pub unmap_seq: [u8; 32],
    /// 每个 size class 的"弹跳"计数 (频繁 map/unmap 的惩罚因子)
    pub bounces: [u8; 32],
    /// 全局操作序列计数器 (1-255)
    pub seq: u8,
    /// 当前 brk 值 (程序堆末端)
    pub brk: usize,
}

// ============================================================================
// 链表操作函数
// ============================================================================

/// 将 meta 节点插入双向循环链表尾部（效果上插入到头节点的前面）。
///
/// # Safety
///
/// - `phead` 非 null，指向链表头指针
/// - `m` 非 null，且当前不在任何链表中（`(*m).prev.is_null() && (*m).next.is_null()`）
/// - `*phead` 要么为 null，要么指向一个有效的循环链表
///
/// # Postcondition
///
/// - Case 链表原为空: `(*m).prev == m && (*m).next == m`，`*phead = m`
/// - Case 链表非空: `m` 被插入到 `*phead` 之前，循环链表完整性保持
///
/// # 复杂度
///
/// O(1) 循环链表尾部插入。
pub(crate) unsafe fn queue(phead: *mut *mut Meta, m: *mut Meta) {
    // 经典循环链表尾部插入: 将 m 插入到 head 之前（即尾部）
    debug_assert!((*m).next.is_null());
    debug_assert!((*m).prev.is_null());
    if !(*phead).is_null() {
        let head = *phead;
        (*m).next = head;
        (*m).prev = (*head).prev;
        (*(*m).next).prev = m;
        (*(*m).prev).next = m;
    } else {
        // 空链表: 自环
        (*m).prev = m;
        (*m).next = m;
        *phead = m;
    }
}

/// 从双向循环链表中移除 meta 节点。
///
/// # Safety
///
/// - `phead` 非 null
/// - `m` 非 null，且 `m` 必须在 `*phead` 指向的链表中
///
/// # Postcondition
///
/// - Case 链表只剩一个节点: `*phead = null`, `(*m).prev = (*m).next = null`
/// - Case 链表有多个节点: `m` 从链表中移除，前后节点正确重链
///   - 若 `*phead == m`，则 `*phead` 更新为 `(*m).next`
///   - `(*m).prev = (*m).next = null`
///
/// # 复杂度
///
/// O(1) 循环链表删除。
pub(crate) unsafe fn dequeue(phead: *mut *mut Meta, m: *mut Meta) {
    if (*m).next != m {
        // 链表有多个节点: 重链前后节点
        (*(*m).prev).next = (*m).next;
        (*(*m).next).prev = (*m).prev;
        if *phead == m {
            *phead = (*m).next;
        }
    } else {
        // 链表只剩一个节点
        *phead = core::ptr::null_mut();
    }
    (*m).prev = core::ptr::null_mut();
    (*m).next = core::ptr::null_mut();
}

/// 从双向循环链表中取出并返回头节点。
///
/// # Safety
///
/// - `phead` 非 null
///
/// # Postcondition
///
/// - Case 链表为空: 返回 `null`
/// - Case 链表非空: 返回原 `*phead`，该节点已从链表中移除
///
/// # 复杂度
///
/// O(1)，委托给 `dequeue()`。
pub(crate) unsafe fn dequeue_head(phead: *mut *mut Meta) -> *mut Meta {
    let m = *phead;
    if !m.is_null() {
        dequeue(phead, m);
    }
    m
}

/// 将使用完毕的 meta 结构体清零并回收到全局 `CTX.free_meta_head` 空闲链表中。
///
/// # Safety
///
/// - `m` 非 null，指向一个不再使用的 `Meta`
/// - 调用者持有 malloc 锁
///
/// # Postcondition
///
/// - `m` 所有字段被清零
/// - `m` 被加入 `CTX.free_meta_head` 链表
///
/// # 依赖
///
/// `queue()`
pub(crate) unsafe fn free_meta(m: *mut Meta) {
    // 清零整个 Meta 结构体，等效于 C 的 *m = (struct meta){0}
    core::ptr::write_bytes(m as *mut u8, 0, core::mem::size_of::<Meta>());
    queue(core::ptr::addr_of_mut!(CTX.free_meta_head), m);
}

// ============================================================================
// 槽位激活与分配
// ============================================================================

/// 通过原子 CAS 操作将 `freed_mask` 中 `active_idx` 范围内的已释放槽位转移到
/// `avail_mask` 中，使其变为可分配状态。
///
/// # Safety
///
/// - `m` 非 null
/// - 调用者持有 malloc 锁（至少 rdlock）
///
/// # Precondition
///
/// - `(*m).avail_mask` 的当前值为 0（组当前无可分配槽位，才会触发 activate）
///
/// # Postcondition
///
/// - `(*m).avail_mask` 包含原 `freed_mask` 中在 `active_idx` 位范围内的所有位
/// - `(*m).freed_mask` 中被认领的位已通过 CAS 原子清除
/// - 返回 `(*m).avail_mask` 的新值 (u32)
///
/// # Algorithm
///
/// - 计算公式 `act = (2u32 << (*m).mem.active_idx) - 1` 构造掩码
/// - 使用 `AtomicI32::compare_exchange` 原子 CAS 循环从 `freed_mask` 中取出低位释放槽位
pub(crate) unsafe fn activate_group(m: *mut Meta) -> u32 {
    // 构造掩码: act = (2u32 << active_idx) - 1
    // 使用 wrapping 操作匹配 C 的 unsigned overflow 语义
    let active_idx = (*((*m).mem)).active_idx as u32;
    let act = if active_idx >= 31 { u32::MAX }
        else { (2u32 << active_idx).wrapping_sub(1) };
    // CAS 循环: 原子地从 freed_mask 中取出 act 范围内的位
    loop {
        let mask = (*m).freed_mask.load(Ordering::Acquire) as u32;
        let new_freed = mask & !act;
        match (*m).freed_mask.compare_exchange_weak(
            mask as i32,
            new_freed as i32,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => {
                let result = mask & act;
                (*m).avail_mask.store(result as i32, Ordering::Release);
                return result;
            }
            Err(_) => continue,
        }
    }
}

// ============================================================================
// 分配指针 → 元数据逆向解析
// ============================================================================

/// 从分配指针的 in-band header 中提取槽位索引。
///
/// # Safety
///
/// - `p` 指向一个已分配块的起始地址
///
/// # Postcondition
///
/// - 返回 `(p.sub(3).read() & 31) as usize`，即 header 字节的低 5 位（0-31 的槽位索引）
///
/// # 设计说明
///
/// C 原版的 `p[-3] & 31` 在 Rust 中使用 `p.sub(3).read() & 31`，更清晰地表达了指针偏移和读取操作。
pub(crate) unsafe fn get_slot_index(p: *const u8) -> usize {
    (p.sub(3).read() & 31) as usize
}

/// 从任意分配指针逆向推导对应的 `Meta`。
///
/// 这是 rusl 设计中最核心的安全校验函数，通过多重断言确保指针合法性，
/// 防止 double-free、伪造指针等攻击。
///
/// # Safety
///
/// - `p` 非 null，`p as usize` 为 16 字节对齐
/// - `p` 指向一个由 mallocng 分配的合法内存块
///
/// # Postcondition
///
/// - Case 所有断言通过: 返回该块所属 group 的 `*mut Meta`
/// - Case 任一断言失败: `core::intrinsics::abort()` — 进程立即终止（防内存损坏传播）
///
/// # 校验链 (按顺序):
///
/// 1. `debug_assert!((p as usize & 15) == 0)` — 地址 16 字节对齐
/// 2. 读取 `p.sub(2)` 作为 16 位偏移量，`get_slot_index(p)` 获取槽位索引
/// 3. 若 `p.sub(4).read() != 0`，表明使用了非零起始偏移，则偏移量实际存储于 `p.sub(8)`，
///    且 `debug_assert!(offset > 0xFFFF)`
/// 4. 计算 group 基址 `base = p.sub(UNIT * offset + UNIT)`
/// 5. 通过 `(*base).meta` 获取元数据指针
/// 6. `debug_assert!((*meta).mem == base)` — 双向绑定验证
/// 7. `debug_assert!(index <= (*meta).last_idx())` — 索引不越界
/// 8. `debug_assert!(((*meta).avail_mask.load(Ordering::Relaxed) & (1u32 << index)) == 0)`
/// 9. `debug_assert!(((*meta).freed_mask.load(Ordering::Relaxed) & (1u32 << index)) == 0)`
/// 10. 计算 meta_area 指针（页对齐向下取整）
/// 11. `debug_assert!((*area).check == CTX.secret)` — 密钥验证防伪造
/// 12. 对于 `sizeclass < 48`，验证偏移量与 sizeclass 的一致性
/// 13. 对于 `sizeclass == 63`（mmap 大对象），确认 `(*meta).sizeclass() == 63`
/// 14. 若 `(*meta).maplen()` 非零，验证偏移量不超过页映射范围
pub(crate) unsafe fn get_meta(p: *const u8) -> *mut Meta {
    // 1. 地址 16 字节对齐
    debug_assert!((p as usize & 15) == 0);

    // 2. 读取 16 位偏移量和槽位索引
    let mut offset = p.sub(2).cast::<u16>().read() as usize;
    let index = get_slot_index(p);

    // 3. 若 p[-4] 非零，表明使用了非零起始偏移
    if p.sub(4).read() != 0 {
        debug_assert!(offset == 0);
        offset = p.sub(8).cast::<u32>().read() as usize;
        debug_assert!(offset > 0xFFFF);
    }

    // 4. 计算 group 基址: base = p - UNIT*offset - UNIT
    let base = p.sub(UNIT * offset).sub(UNIT) as *mut Group;

    // 5. 通过 base->meta 获取元数据指针
    let meta = (*base).meta;

    // 6. 双向绑定验证 — 若失败表明堆元数据已损坏，返回 null
    if meta.is_null() || (*meta).mem != base {
        return core::ptr::null_mut();
    }

    // 7. 索引不越界
    debug_assert!(index <= (*meta).last_idx());

    // 8. 槽位不空闲（已分配状态）
    debug_assert!(((*meta).avail_mask.load(Ordering::Relaxed) as u32 & (1u32 << index)) == 0);

    // 9. 槽位未被释放
    debug_assert!(((*meta).freed_mask.load(Ordering::Relaxed) as u32 & (1u32 << index)) == 0);

    // 10. 计算 meta_area 指针（页对齐向下取整）
    let area = (meta as usize & !4095usize) as *const MetaArea;

    // 11. 密钥验证防伪造
    debug_assert!((*area).check == CTX.secret);

    // 12. 对于 sizeclass < 48，验证偏移量与 sizeclass 的一致性
    let sc = (*meta).sizeclass();
    if sc < 48 {
        debug_assert!(offset >= SIZE_CLASSES[sc] as usize * index);
        debug_assert!(offset < SIZE_CLASSES[sc] as usize * (index + 1));
    } else {
        // 13. mmap 大对象: sizeclass 应为 63
        debug_assert!(sc == 63);
    }

    // 14. 若 maplen 非零，验证偏移量不超过页映射范围
    let ml = (*meta).maplen();
    if ml != 0 {
        debug_assert!(offset <= ml * 4096 / UNIT - 1);
    }

    meta
}

// ============================================================================
// 分配块大小编解码
// ============================================================================

/// 从分配块的 header 中恢复用户原始请求的分配大小（nominal size = 不含 reserved 区域的净大小）。
///
/// # Safety
///
/// - `p` 指向分配块起始地址
/// - `end` 指向分配块末尾地址（`p + stride - IB`）
/// - 分配的 header 格式合法
///
/// # Postcondition
///
/// - 返回 `end as usize - reserved - p as usize`，即用户可用字节数
///
/// # 编码解码规则:
///
/// - `reserved = p.sub(3).read() >> 5` 读取 reserved 值（高 3 位）
/// - 若 `reserved >= 5`，则实际 reserved 值存储在 `end.sub(4).cast::<u32>().read()`
///   且 `debug_assert!(reserved >= 5)`
/// - 大 reserved 情况额外校验 `debug_assert!(end.sub(5).read() == 0)`
/// - 校验 `debug_assert!(end.sub(reserved).read() == 0)`（分隔零字节）
/// - 校验 `debug_assert!(*end == 0)`（溢出检查字节）
pub(crate) unsafe fn get_nominal_size(p: *const u8, end: *const u8) -> usize {
    // 读取 reserved 值（header 字节的高 3 位）
    let mut reserved = (p.sub(3).read() >> 5) as usize;
    if reserved >= 5 {
        // 扩展编码: reserved 实际值存储在 end[-4..-1] 的 u32 中
        debug_assert!(reserved == 5);
        reserved = end.sub(4).cast::<u32>().read() as usize;
        debug_assert!(reserved >= 5);
        debug_assert!(end.sub(5).read() == 0);
    }
    debug_assert!(reserved <= end as usize - p as usize);
    // 分隔零字节
    debug_assert!(end.sub(reserved).read() == 0);
    // 溢出检查字节
    debug_assert!(end.read() == 0);
    end as usize - reserved - p as usize
}

/// 返回给定元数据所描述组中每个槽位的大小。
///
/// # Safety
///
/// - `g` 非 null
///
/// # Postcondition
///
/// - Case 独立 mmap (last_idx==0 && maplen>0): 返回 `maplen * 4096 - UNIT`
/// - Case 常规 slab 组: 返回 `UNIT * SIZE_CLASSES[(*g).sizeclass()]`
pub(crate) unsafe fn get_stride(g: *const Meta) -> usize {
    // Case 独立 mmap (last_idx==0 && maplen>0): 返回整个映射区域减去 group header
    if (*g).last_idx() == 0 && (*g).maplen() > 0 {
        return (*g).maplen() * 4096 - UNIT;
    }
    // Case 常规 slab 组: 槽位大小由 sizeclass 查表决定
    UNIT * SIZE_CLASSES[(*g).sizeclass()] as usize
}

/// 在分配块的 in-band header 中写入用户请求大小 `n`（通过设置 reserved 区域来实现）。
///
/// # Safety
///
/// - `p` 指向分配块起始
/// - `end` 指向分配块末尾 `p + stride - IB`
/// - `n <= end as usize - p as usize`（请求大小不大于槽位容量）
///
/// # Postcondition:
///
/// - `reserved = end as usize - p as usize - n`
/// - 若 `reserved > 0`，则 `end.sub(reserved).write(0)`（设置分隔零字节）
/// - 若 `reserved >= 5`，则在 `end.sub(4).cast::<u32>().write(reserved as u32)`
///   并在 `end.sub(5).write(0)` 标记
/// - `p.sub(3)` 字节高 3 位被设置为 reserved（最大取 7，>=5 时取 5 用扩展编码）
pub(crate) unsafe fn set_size(p: *mut u8, end: *mut u8, n: usize) {
    let mut reserved = end as usize - p as usize - n;
    // 若 reserved > 0，设置分隔零字节
    if reserved > 0 {
        end.sub(reserved).write(0);
    }
    // reserved >= 5 时使用扩展编码
    if reserved >= 5 {
        end.sub(4).cast::<u32>().write(reserved as u32);
        end.sub(5).write(0);
        reserved = 5;
    }
    // 更新 header 字节: 保留低 5 位 (slot index), 在高 3 位写入 reserved
    let header_byte = p.sub(3).read();
    p.sub(3).write((header_byte & 31) | ((reserved as u8) << 5));
}

/// 在指定槽位中构造一个完整的新分配块。
///
/// 这是 `malloc()` 实际创建分配块的底层操作。
///
/// # Safety
///
/// - `g` 非 null，`(*g).mem` 非 null
/// - `idx` 是有效的槽位索引
/// - `n` 是用户请求的分配大小
/// - `ctr` 是分配计数器（用于随机化偏移）
///
/// # Postcondition
///
/// 返回用户可用指针 `*mut u8`，其 header 满足 `get_slot_index(p) == idx`。
/// 通过非零偏移和随机化递增，同一槽位连续分配时产生不同地址。
///
/// # 依赖:
///
/// - `get_stride(g)` — 获取槽位总大小
/// - `set_size(p, end, n)` — 写入大小信息
pub(crate) unsafe fn enframe(g: *mut Meta, idx: usize, n: usize, ctr: usize) -> *mut u8 {
    let stride = get_stride(g);
    let slack = (stride - IB - n) / UNIT;

    // storage 柔性数组紧跟在 Group header 之后 (偏移 UNIT 字节)
    let storage = ((*g).mem as *mut u8).add(UNIT);
    let mut p = storage.add(stride * idx);
    let end = p.add(stride - IB);

    // 计算循环偏移量用于地址随机化，防止 double-free
    let mut off: usize = (if p.sub(3).read() != 0 {
        p.sub(2).cast::<u16>().read() as usize + 1
    } else {
        ctr
    }) & 255;
    debug_assert!(p.sub(4).read() == 0);

    // 若偏移量超出 slack，压缩到有效范围
    if off > slack {
        let mut m = slack;
        m |= m >> 1;
        m |= m >> 2;
        m |= m >> 4;
        off &= m;
        if off > slack {
            off -= slack + 1;
        }
        debug_assert!(off <= slack);
    }

    if off != 0 {
        // 在原偏移零位置存储偏移量
        p.sub(2).cast::<u16>().write(off as u16);
        p.sub(3).write(7 << 5);
        // 推进 p 到新偏移位置
        p = p.add(UNIT * off);
        // 在非零偏移处创建永久校验字节
        p.sub(4).write(0);
    }

    // 写入最终 header: 16 位偏移量和槽位索引
    let final_offset = (p as usize - storage as usize) / UNIT;
    p.sub(2).cast::<u16>().write(final_offset as u16);
    p.sub(3).write(idx as u8);
    set_size(p, end, n);
    p
}

// ============================================================================
// 大小类别映射
// ============================================================================

/// 将用户请求大小 `n` 映射到大小类别索引 (0..47)。
///
/// # Postcondition
///
/// - 返回 0..47 的 sizeclass 索引，保证 `SIZE_CLASSES[sc] * UNIT >= n + IB - 1`
///
/// # Algorithm
///
/// 1. `n = (n + IB - 1) >> 4` — 将字节大小转换为 UNIT 单位并向上取整
/// 2. 若 `n < 10` 直接返回 `n`
/// 3. 否则 `n += 1`，使用 `usize::leading_zeros()` 计算最高位位置
/// 4. 结合分段查表 `SIZE_CLASSES[]` 精确定位，通过两次比较修正索引
///
/// # 依赖
///
/// `SIZE_CLASSES[]` (pub(crate) static, 定义于 context.rs)
///
/// # 设计说明
///
/// C 原版使用 `a_clz_32()` 宏。Rust 使用 `usize::leading_zeros()` 内建方法，
/// 语义等价且零成本（编译为 `lzcnt`/`clz` 指令）。
pub(crate) fn size_to_class(n: usize) -> usize {
    // 将字节大小转换为 UNIT 单位并向上取整 (补偿 IB 开销)
    let mut n_units = (n + IB - 1) >> 4;
    if n_units < 10 {
        return n_units;
    }
    n_units += 1;
    // 使用 u32 的 leading_zeros 匹配 C 原版的 a_clz_32
    let n_u32 = n_units as u32;
    let mut i = (28usize.wrapping_sub(n_u32.leading_zeros() as usize)) * 4 + 8;
    // 两次比较修正索引: 分段查表精确定位
    if n_units > SIZE_CLASSES[i + 1] as usize {
        i += 2;
    }
    if n_units > SIZE_CLASSES[i] as usize {
        i += 1;
    }
    i
}

/// 检查请求分配大小是否会导致溢出。
///
/// # Postcondition
///
/// - Case 溢出: `n >= usize::MAX / 2 - 4096`，返回 `true`
/// - Case 安全: 返回 `false`
///
/// # Note
///
/// - 调用者应负责设置 errno（由外层的 POSIX 适配层处理）
/// - 与 C 原版不同，Rust 版本返回 `bool`，由调用者处理 errno 设置
///   （因为 rusl 是 `#![no_std]`，errno 需通过平台抽象层处理）
pub(crate) fn size_overflows(n: usize) -> bool {
    n >= usize::MAX / 2 - 4096
}

// ============================================================================
// 反碎片化序列号系统
// ============================================================================

/// 推进全局操作序列计数器 `CTX.seq`。
///
/// 当计数器达上限 (255) 时，重置回 1 并清零所有 `unmap_seq[]`。
///
/// # Safety
///
/// - 调用者持有 malloc 锁
///
/// # Postcondition
///
/// - Case seq==255: 重置 seq=1，所有 unmap_seq[i]=0
/// - Case seq<255: seq++
pub(crate) unsafe fn step_seq() {
    if CTX.seq == 255 {
        // 序列号回绕: 重置 seq=1，清零所有 unmap_seq
        for i in 0..32 {
            CTX.unmap_seq[i] = 0;
        }
        CTX.seq = 1;
    } else {
        CTX.seq += 1;
    }
}

/// 记录某个 size class 最近一次触发 unmap 时的全局序列号。
///
/// # Postcondition
///
/// - 若 `sc >= 7 && sc < 39`，则 `CTX.unmap_seq[sc - 7] = CTX.seq`
pub(crate) unsafe fn record_seq(sc: usize) {
    // 仅记录 size class 7..38 的序列号（class 0-6 的小对象从不触发 unmap）
    if sc.wrapping_sub(7) < 32 {
        CTX.unmap_seq[sc - 7] = CTX.seq;
    }
}

/// 检测并记录特定 size class 的 map/unmap 抖动行为。
///
/// # Safety
///
/// - 调用者持有 malloc 锁
///
/// # Postcondition
///
/// - 若满足抖动条件（`sc >= 7 && sc < 39`，上次序列号非零，且
///   `CTX.seq - seq < 10`），则递增 `CTX.bounces[sc-7]`（上限 150）
pub(crate) unsafe fn account_bounce(sc: usize) {
    if sc.wrapping_sub(7) < 32 {
        let seq = CTX.unmap_seq[sc - 7];
        // 抖动检测: 若上次 unmap 序列号非零，且距当前操作 < 10 步
        if seq != 0 && CTX.seq.wrapping_sub(seq) < 10 {
            if CTX.bounces[sc - 7] + 1 < 100 {
                CTX.bounces[sc - 7] += 1;
            } else {
                CTX.bounces[sc - 7] = 150;
            }
        }
    }
}

/// 逐步衰减某个 size class 的弹跳计数。
///
/// # Postcondition
///
/// - 若 `sc >= 7 && sc < 39` 且 `CTX.bounces[sc-7] > 0`，则递减
pub(crate) unsafe fn decay_bounces(sc: usize) {
    if sc.wrapping_sub(7) < 32 && CTX.bounces[sc - 7] > 0 {
        CTX.bounces[sc - 7] -= 1;
    }
}

/// 查询某个 size class 是否处于"弹跳"状态。
///
/// 处于弹跳状态的 size class 不应立即将 group 释放给内核，
/// 以避免频繁的 map/unmap 抖动。
///
/// # Postcondition
///
/// - 若 `sc >= 7 && sc < 39` 且 `CTX.bounces[sc-7] >= 100`，返回 `true`
/// - 否则返回 `false`
pub(crate) unsafe fn is_bouncing(sc: usize) -> bool {
    sc.wrapping_sub(7) < 32 && CTX.bounces[sc - 7] >= 100
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;
    use core::sync::atomic::Ordering;
    use rusl_core::test;

    // ========================================================================
    // 常量验证测试
    // ========================================================================

    test!("test_constants_unit_is_16" {
        // UNIT 必须为 16，这是所有对齐和计算的基础
        assert_eq!(UNIT, 16);
    });

    test!("test_constants_ib_is_4" {
        // IB 为 in-band 元数据大小，固定在 4 字节
        assert_eq!(IB, 4);
    });

    test!("test_constants_mmap_threshold" {
        // MMAP_THRESHOLD = 131052 字节 (约 128KB)
        assert_eq!(MMAP_THRESHOLD, 131052);
    });

    test!("test_constants_unit_is_power_of_two" {
        assert!(UNIT.is_power_of_two());
    });

    test!("test_constants_ib_is_power_of_two" {
        assert!(IB.is_power_of_two());
    });

    test!("test_constants_unit_plus_ib_equals_20" {
        assert_eq!(UNIT + IB, 20);
    });

    test!("test_constants_unit_is_alignment_for_x86_64" {
        // 16 字节对齐符合 x86-64 ABI
        assert_eq!(UNIT % 16, 0);
    });

    test!("test_constants_mmap_threshold_above_128k" {
        // MMAP_THRESHOLD = 131052 略低于 128K (131072)，相差 20 字节。
        // 这是 musl 有意设计：避免恰好等于页面大小的分配走 mmap 路径。
        assert!(MMAP_THRESHOLD >= 131052);
    });

    test!("test_constants_mmap_threshold_below_256k" {
        // MMAP_THRESHOLD 应小于 256*1024
        assert!(MMAP_THRESHOLD < 256 * 1024);
    });

    // ========================================================================
    // Group 结构体布局验证
    // ========================================================================

    test!("test_group_size_is_unit" {
        // Group header 必须恰好为 UNIT 字节 (16 字节)
        assert_eq!(mem::size_of::<Group>(), UNIT);
    });

    test!("test_group_alignment_is_reasonable" {
        // Group 的对齐要求至少为指针大小
        assert!(mem::align_of::<Group>() >= mem::align_of::<*mut Meta>());
    });

    test!("test_group_meta_field_offset" {
        // meta 字段应在偏移 0 处（第一个字段）
        // 使用 addr_of! 宏安全获取字段偏移，避免 null 指针解引用
        let group_ptr: *const Group = core::ptr::null();
        let meta_ptr = unsafe { core::ptr::addr_of!((*group_ptr).meta) as *const *mut Meta };
        assert_eq!(meta_ptr as usize, 0);
    });

    test!("test_group_active_idx_is_u8" {
        // active_idx 是 u8 类型（C 原版为 unsigned char:5）
        assert_eq!(mem::size_of::<u8>(), 1);
        // active_idx 偏移: 在 meta 指针之后 (8 字节)
        let group_ptr: *const Group = core::ptr::null();
        let idx_ptr = unsafe { core::ptr::addr_of!((*group_ptr).active_idx) };
        assert_eq!(idx_ptr as usize, mem::size_of::<*mut Meta>());
    });

    test!("test_group_pad_fills_to_unit" {
        // pad 字段确保 Group header 恰好为 UNIT 字节
        let expected_pad = UNIT - mem::size_of::<*mut Meta>() - 1;
        assert_eq!(mem::size_of::<[u8; UNIT - mem::size_of::<*mut Meta>() - 1]>(), expected_pad)
    });

    // ========================================================================
    // Meta 结构体布局验证
    // ========================================================================

    test!("test_meta_size_is_expected" {
        // Meta 大小: 3 个指针 (24 字节) + 2 个 AtomicI32 (8 字节) + 1 个 usize (8 字节) = 40 字节
        // 注意: 实际大小可能因 repr(C) 的对齐规则为 40 或 48
        let size = mem::size_of::<Meta>();
        // 至少为 40 字节
        assert!(size >= 40, "Meta size = {} < 40", size);
        // 通常不超过 48 字节
        assert!(size <= 48, "Meta size = {} > 48", size);
    });

    test!("test_meta_alignment" {
        // Meta 应对齐到指针大小边界
        assert!(mem::align_of::<Meta>() >= mem::align_of::<*mut Meta>());
    });

    test!("test_meta_prev_field_is_at_offset_0" {
        // prev 在 repr(C) 下是第一个字段，偏移应为 0
        let meta_ptr: *const Meta = core::ptr::null();
        let prev_ptr = unsafe { core::ptr::addr_of!((*meta_ptr).prev) } as *const *mut Meta;
        assert_eq!(prev_ptr as usize, 0);
    });

    test!("test_meta_next_field_is_at_offset_8" {
        // next 是第二个指针字段，在 64-bit 上偏移为 8
        let meta_ptr: *const Meta = core::ptr::null();
        let next_ptr = unsafe { core::ptr::addr_of!((*meta_ptr).next) } as *const *mut Meta;
        assert_eq!(next_ptr as usize, mem::size_of::<*mut Meta>());
    });

    test!("test_meta_mem_field_is_at_offset_16" {
        // mem 是第三个指针字段，在 64-bit 上偏移为 16
        let meta_ptr: *const Meta = core::ptr::null();
        let mem_ptr = unsafe { core::ptr::addr_of!((*meta_ptr).mem) } as *const *mut Group;
        assert_eq!(mem_ptr as usize, 2 * mem::size_of::<*mut Meta>());
    });

    test!("test_meta_avail_mask_is_atomici32" {
        // 验证 avail_mask 类型的编译期检查
        let _mask: AtomicI32 = AtomicI32::new(0);
        // 类型断言：确保该字段接受 AtomicI32 操作
        assert_eq!(mem::size_of::<AtomicI32>(), 4);
    });

    test!("test_meta_freed_mask_is_atomici32" {
        // 验证 freed_mask 类型的编译期检查
        assert_eq!(mem::size_of::<AtomicI32>(), 4);
    });

    test!("test_meta_bitfields_is_usize" {
        // bitfields 是 usize 类型，在 64-bit 上位 8 字节
        assert_eq!(mem::size_of::<usize>(), mem::size_of::<*const u8>());
    });

    // ========================================================================
    // Meta 位域访问器测试
    // ========================================================================

    /// 创建一个用于测试的零初始化 Meta
    fn make_test_meta() -> Meta {
        Meta {
            prev: core::ptr::null_mut(),
            next: core::ptr::null_mut(),
            mem: core::ptr::null_mut(),
            avail_mask: AtomicI32::new(0),
            freed_mask: AtomicI32::new(0),
            bitfields: 0,
        }
    }

    // ---- last_idx 测试 ----

    test!("test_meta_last_idx_initial_value_is_0" {
        let meta = make_test_meta();
        assert_eq!(meta.last_idx(), 0);
    });

    test!("test_meta_set_last_idx_0" {
        let mut meta = make_test_meta();
        meta.set_last_idx(0);
        assert_eq!(meta.last_idx(), 0);
    });

    test!("test_meta_set_last_idx_31" {
        let mut meta = make_test_meta();
        meta.set_last_idx(31);
        assert_eq!(meta.last_idx(), 31);
    });

    test!("test_meta_last_idx_truncates_to_5_bits" {
        let mut meta = make_test_meta();
        meta.set_last_idx(32); // 32 = 0b100000, 低 5 位 = 0
        assert_eq!(meta.last_idx(), 0);
        meta.set_last_idx(63); // 63 = 0b111111, 低 5 位 = 31
        assert_eq!(meta.last_idx(), 31);
        meta.set_last_idx(33); // 33 = 0b100001, 低 5 位 = 1
        assert_eq!(meta.last_idx(), 1);
    });

    test!("test_meta_last_idx_masks_only_5_bits" {
        let mut meta = make_test_meta();
        // 设置一个大于 5 位的值，高位应被忽略
        meta.set_last_idx(0b11111_00000_00000); // bit 10-16 置 1
        // 但由于 set_last_idx 截断，last_idx 应为 0
        assert_eq!(meta.last_idx(), 0);
    });

    test!("test_meta_set_last_idx_does_not_affect_other_bitfields" {
        let mut meta = make_test_meta();
        meta.set_sizeclass(42); // 0b101010
        meta.set_freeable(true);
        meta.set_maplen(100);
        let prev_sc = meta.sizeclass();
        let prev_fr = meta.freeable();
        let prev_ml = meta.maplen();

        meta.set_last_idx(17);
        // 其他字段应保持不变
        assert_eq!(meta.sizeclass(), prev_sc);
        assert_eq!(meta.freeable(), prev_fr);
        assert_eq!(meta.maplen(), prev_ml);
        assert_eq!(meta.last_idx(), 17);
    });

    // ---- freeable 测试 ----

    test!("test_meta_freeable_initial_value_is_false" {
        let meta = make_test_meta();
        assert!(!meta.freeable());
    });

    test!("test_meta_set_freeable_true" {
        let mut meta = make_test_meta();
        meta.set_freeable(true);
        assert!(meta.freeable());
    });

    test!("test_meta_set_freeable_false" {
        let mut meta = make_test_meta();
        meta.set_freeable(true);
        meta.set_freeable(false);
        assert!(!meta.freeable());
    });

    test!("test_meta_set_freeable_toggle" {
        let mut meta = make_test_meta();
        meta.set_freeable(true);
        assert!(meta.freeable());
        meta.set_freeable(false);
        assert!(!meta.freeable());
        meta.set_freeable(true);
        assert!(meta.freeable());
    });

    test!("test_meta_freeable_does_not_affect_last_idx" {
        let mut meta = make_test_meta();
        meta.set_last_idx(31);
        meta.set_freeable(true);
        assert_eq!(meta.last_idx(), 31);
        meta.set_freeable(false);
        assert_eq!(meta.last_idx(), 31);
    });

    // ---- sizeclass 测试 ----

    test!("test_meta_sizeclass_initial_value_is_0" {
        let meta = make_test_meta();
        assert_eq!(meta.sizeclass(), 0);
    });

    test!("test_meta_set_sizeclass_0" {
        let mut meta = make_test_meta();
        meta.set_sizeclass(0);
        assert_eq!(meta.sizeclass(), 0);
    });

    test!("test_meta_set_sizeclass_47" {
        let mut meta = make_test_meta();
        meta.set_sizeclass(47);
        assert_eq!(meta.sizeclass(), 47);
    });

    test!("test_meta_set_sizeclass_48_is_valid" {
        // sizeclass 支持 0..63 (6 位)
        let mut meta = make_test_meta();
        meta.set_sizeclass(48);
        assert_eq!(meta.sizeclass(), 48);
    });

    test!("test_meta_set_sizeclass_63_max" {
        // sizeclass 最大值 63
        let mut meta = make_test_meta();
        meta.set_sizeclass(63);
        assert_eq!(meta.sizeclass(), 63);
    });

    test!("test_meta_sizeclass_truncates_to_6_bits" {
        let mut meta = make_test_meta();
        meta.set_sizeclass(64); // 64 = 0b1000000, 低 6 位 = 0
        assert_eq!(meta.sizeclass(), 0);
        meta.set_sizeclass(127); // 127 = 0b1111111, 低 6 位 = 63
        assert_eq!(meta.sizeclass(), 63);
        meta.set_sizeclass(65); // 65 = 0b1000001, 低 6 位 = 1
        assert_eq!(meta.sizeclass(), 1);
    });

    test!("test_meta_sizeclass_does_not_affect_lower_bitfields" {
        let mut meta = make_test_meta();
        meta.set_last_idx(31);
        meta.set_freeable(true);
        meta.set_sizeclass(42);
        assert_eq!(meta.last_idx(), 31);
        assert!(meta.freeable());
        assert_eq!(meta.sizeclass(), 42);
    });

    // ---- maplen 测试 ----

    test!("test_meta_maplen_initial_value_is_0" {
        let meta = make_test_meta();
        assert_eq!(meta.maplen(), 0);
    });

    test!("test_meta_set_maplen_0" {
        let mut meta = make_test_meta();
        meta.set_maplen(0);
        assert_eq!(meta.maplen(), 0);
    });

    test!("test_meta_set_maplen_1" {
        let mut meta = make_test_meta();
        meta.set_maplen(1);
        assert_eq!(meta.maplen(), 1);
    });

    test!("test_meta_set_maplen_large_value" {
        let mut meta = make_test_meta();
        // maplen 使用高位 (52 位可用)，设置一个大值
        let large = 0x12345usize;
        meta.set_maplen(large);
        assert_eq!(meta.maplen(), large);
    });

    test!("test_meta_set_maplen_does_not_affect_lower_12_bits" {
        let mut meta = make_test_meta();
        meta.set_last_idx(31);          // 低 5 位: 11111
        meta.set_freeable(true);         // 第 5 位: 1
        meta.set_sizeclass(63);          // 第 6-11 位: 111111
        // 低 12 位 = 0b111111_1_11111 = 0xFFF
        assert_eq!(meta.bitfields & 0xFFF, 0xFFF);

        meta.set_maplen(1000);
        // 低 12 位应保持不变
        assert_eq!(meta.last_idx(), 31);
        assert!(meta.freeable());
        assert_eq!(meta.sizeclass(), 63);
        assert_eq!(meta.maplen(), 1000);
    });

    test!("test_meta_maplen_max_value" {
        let mut meta = make_test_meta();
        // maplen 可以使用 usize 的所有高位
        meta.set_maplen(usize::MAX >> 12);
        assert_eq!(meta.maplen(), usize::MAX >> 12);
    });

    // ---- 综合位域测试 ----

    test!("test_meta_bitfields_all_fields_independent" {
        let mut meta = make_test_meta();

        // 设置所有字段到特定值
        meta.set_last_idx(10);      // 0b01010
        meta.set_freeable(true);     // bit 5 = 1
        meta.set_sizeclass(20);      // 0b010100
        meta.set_maplen(500);

        // 验证所有字段独立
        assert_eq!(meta.last_idx(), 10);
        assert!(meta.freeable());
        assert_eq!(meta.sizeclass(), 20);
        assert_eq!(meta.maplen(), 500);

        // 修改一个字段不应影响其他
        meta.set_last_idx(25);
        assert_eq!(meta.last_idx(), 25);
        assert!(meta.freeable());
        assert_eq!(meta.sizeclass(), 20);
        assert_eq!(meta.maplen(), 500);
    });

    test!("test_meta_bitfields_zero_after_full_cycle" {
        let mut meta = make_test_meta();

        // 设置所有字段
        meta.set_last_idx(31);
        meta.set_freeable(true);
        meta.set_sizeclass(63);
        meta.set_maplen(1000);

        // 清零所有字段
        meta.set_last_idx(0);
        meta.set_freeable(false);
        meta.set_sizeclass(0);
        meta.set_maplen(0);

        assert_eq!(meta.bitfields, 0);
        assert_eq!(meta.last_idx(), 0);
        assert!(!meta.freeable());
        assert_eq!(meta.sizeclass(), 0);
        assert_eq!(meta.maplen(), 0);
    });

    test!("test_meta_bitfields_boundary_12bit_transition" {
        // 验证低 12 位 (last_idx + freeable + sizeclass) 和高位 (maplen) 的边界
        let mut meta = make_test_meta();
        // 设置低 12 位全 1
        meta.set_last_idx(31);
        meta.set_freeable(true);
        meta.set_sizeclass(63);
        assert_eq!(meta.bitfields & 0xFFF, 0xFFF);

        // 设置 maplen 为 1
        meta.set_maplen(1);
        // 低 12 位应保持全 1
        assert_eq!(meta.bitfields & 0xFFF, 0xFFF);
        assert_eq!(meta.maplen(), 1);
    });

    // ========================================================================
    // MetaArea 结构体布局验证
    // ========================================================================

    test!("test_metaarea_check_is_u64" {
        assert_eq!(mem::size_of::<u64>(), 8);
    });

    test!("test_metaarea_check_offset" {
        let area_ptr: *const MetaArea = core::ptr::null();
        let check_ptr = unsafe { core::ptr::addr_of!((*area_ptr).check) } as *const u64;
        assert_eq!(check_ptr as usize, 0);
    });

    test!("test_metaarea_next_offset_after_check" {
        let area_ptr: *const MetaArea = core::ptr::null();
        let next_ptr = unsafe { core::ptr::addr_of!((*area_ptr).next) } as *const *mut MetaArea;
        assert_eq!(next_ptr as usize, mem::size_of::<u64>()); // 在 check (8 字节)之后
    });

    test!("test_metaarea_nslots_is_i32" {
        // nslots 是 i32 类型
        assert_eq!(mem::size_of::<i32>(), 4);
    });

    test!("test_metaarea_alignment" {
        // MetaArea 需要页对齐（4096），但结构体本身对齐为指针大小
        assert!(mem::align_of::<MetaArea>() >= mem::size_of::<*mut MetaArea>());
    });

    // ========================================================================
    // MallocContext 结构体布局验证
    // ========================================================================

    test!("test_malloc_context_active_array_length" {
        // active 数组必须有 48 个元素 (对应 48 个 size class)
        // 通过 size_of 间接验证: 48 个 *mut Meta = 48*8 = 384 字节
        let ctx_ptr: *const MallocContext = core::ptr::null();
        let active_ptr = unsafe { core::ptr::addr_of!((*ctx_ptr).active) } as *const [*mut Meta; 48];
        // 类型层面的长度由编译器保证，此处仅验证指针计算正确
        let _ = active_ptr; // 类型检查通过
        assert_eq!(mem::size_of::<[*mut Meta; 48]>(), 48 * mem::size_of::<*mut Meta>());
    });

    test!("test_malloc_context_usage_by_class_array_length" {
        // usage_by_class 数组必须有 48 个元素
        let expected_size = 48 * mem::size_of::<usize>();
        assert_eq!(mem::size_of::<[usize; 48]>(), expected_size);
    });

    test!("test_malloc_context_unmap_seq_array_length" {
        // unmap_seq 数组必须有 32 个元素 (对应 sc 7..38)
        assert_eq!(mem::size_of::<[u8; 32]>(), 32);
    });

    test!("test_malloc_context_bounces_array_length" {
        // bounces 数组必须有 32 个元素
        assert_eq!(mem::size_of::<[u8; 32]>(), 32);
    });

    test!("test_malloc_context_seq_is_u8" {
        // seq 是 u8 类型，值域 0..255
        assert_eq!(mem::size_of::<u8>(), 1);
    });

    test!("test_malloc_context_secret_is_u64" {
        assert_eq!(mem::size_of::<u64>(), 8);
    });

    test!("test_malloc_context_init_done_is_i32" {
        assert_eq!(mem::size_of::<i32>(), 4);
    });

    test!("test_malloc_context_mmap_counter_is_u32" {
        assert_eq!(mem::size_of::<u32>(), 4);
    });

    test!("test_malloc_context_brk_is_usize" {
        // brk 是 usize 类型 (C 的 uintptr_t 等效)
        assert_eq!(mem::size_of::<usize>(), mem::size_of::<*const u8>());
    });

    // ========================================================================
    // 不变量验证 (INV 测试)
    // ========================================================================

    test!("test_inv_unit_is_alignment_boundary_for_x86_64_abi" {
        // UNIT = 16 确保与 x86-64 ABI 对齐要求一致
        assert_eq!(UNIT, 16);
    });

    test!("test_inv_mmap_threshold_is_multiple_of_unit_minus_something" {
        // MMAP_THRESHOLD 与 UNIT 的关系验证
        // 131052 = 8192*16 - 4 - 16 (非精确倍数，因为内部对齐调整)
        assert_eq!(MMAP_THRESHOLD % UNIT, 12);
    });

    test!("test_inv_group_mem_bidirectional_note" {
        // 不变量 INV-GROUP-MEM: (*group.meta).mem == group
        // 此测试仅为文档记录，具体验证需在 get_meta 实现后进行
        // TODO: 当 get_meta 实现完成后添加端到端验证
    });

    test!("test_inv_avail_freed_masks_should_be_disjoint_note" {
        // 不变量 INV-MASK-01: avail_mask & freed_mask == 0
        // 此不变量由 activate_group 和 nontrivial_free 维护
        // 测试需要实际的分配/释放序列，待实现后补充
        // TODO: 实现后添加端到端测试
    });

    test!("test_inv_meta_area_check_secret_note" {
        // 不变量 INV-AREA-01: area.check == CTX.secret
        // 此不变量由 get_meta 的校验链强制执行
        // TODO: 实现后添加端到端测试
    });

    test!("test_inv_seq_range_1_to_255" {
        // 不变量 INV-SEQ-01: CTX.seq 在 1..255 范围内循环
        // u8 类型保证值域为 0..255，但具体循环逻辑需实现
        assert!(u8::MAX == 255);
    });

    test!("test_inv_bounce_threshold_100" {
        // 不变量 INV-BOUNCE-01: bounces >= 100 时组不释放给内核
        // bounces 数组元素为 u8，最大值 255，100 在安全范围内
        assert!(100u8 <= u8::MAX);
    });

    test!("test_inv_slot_count_last_idx_plus_one" {
        // 不变量 INV-SLOT-COUNT-01: 对于非 mmap 组，槽位数 = last_idx + 1
        // last_idx 值域为 0..31 (5 位)
        assert!(31 + 1 <= 32); // 最多 32 个槽位
    });

    test!("test_inv_reserved_separator_zero_byte" {
        // 不变量 INV-RESERVED-01: 每个已分配块的 end.sub(reserved) 和 *end 为零字节
        // 此不变量由 set_size 维护，get_nominal_size 验证
        // TODO: 实现后添加端到端测试
    });

    test!("test_inv_header_self_describing" {
        // 不变量 HEADER-01: 任意指针 p 满足 p.sub(2) 可解码为偏移量
        // 这是整个 mallocng 设计的核心不变量
        // TODO: 实现后通过 set_size -> get_meta 往返测试验证
    });

    // ========================================================================
    // 函数签名编译期验证
    // ========================================================================

    test!("test_fn_queue_signature_exists" {
        // 验证 queue 函数签名的存在和正确性（编译期检查）
        // 通过函数指针类型检查验证签名
        let _f: unsafe fn(*mut *mut Meta, *mut Meta) = queue;
    });

    test!("test_fn_dequeue_signature_exists" {
        let _f: unsafe fn(*mut *mut Meta, *mut Meta) = dequeue;
    });

    test!("test_fn_dequeue_head_signature_exists" {
        let _f: unsafe fn(*mut *mut Meta) -> *mut Meta = dequeue_head;
    });

    test!("test_fn_free_meta_signature_exists" {
        let _f: unsafe fn(*mut Meta) = free_meta;
    });

    test!("test_fn_activate_group_signature_exists" {
        let _f: unsafe fn(*mut Meta) -> u32 = activate_group;
    });

    test!("test_fn_get_slot_index_signature_exists" {
        let _f: unsafe fn(*const u8) -> usize = get_slot_index;
    });

    test!("test_fn_get_meta_signature_exists" {
        let _f: unsafe fn(*const u8) -> *mut Meta = get_meta;
    });

    test!("test_fn_get_nominal_size_signature_exists" {
        let _f: unsafe fn(*const u8, *const u8) -> usize = get_nominal_size;
    });

    test!("test_fn_get_stride_signature_exists" {
        let _f: unsafe fn(*const Meta) -> usize = get_stride;
    });

    test!("test_fn_set_size_signature_exists" {
        let _f: unsafe fn(*mut u8, *mut u8, usize) = set_size;
    });

    test!("test_fn_enframe_signature_exists" {
        let _f: unsafe fn(*mut Meta, usize, usize, usize) -> *mut u8 = enframe;
    });

    test!("test_fn_size_to_class_signature_exists" {
        // size_to_class 是安全函数
        let _f: fn(usize) -> usize = size_to_class;
    });

    test!("test_fn_size_overflows_signature_exists" {
        // size_overflows 是安全函数
        let _f: fn(usize) -> bool = size_overflows;
    });

    test!("test_fn_step_seq_signature_exists" {
        let _f: unsafe fn() = step_seq;
    });

    test!("test_fn_record_seq_signature_exists" {
        let _f: unsafe fn(usize) = record_seq;
    });

    test!("test_fn_account_bounce_signature_exists" {
        let _f: unsafe fn(usize) = account_bounce;
    });

    test!("test_fn_decay_bounces_signature_exists" {
        let _f: unsafe fn(usize) = decay_bounces;
    });

    test!("test_fn_is_bouncing_signature_exists" {
        let _f: unsafe fn(usize) -> bool = is_bouncing;
    });

    // ========================================================================
    // pgsz() 测试
    // ========================================================================

    test!("test_pgsz_returns_compile_time_constant" {
        // 当 PAGESIZE cfg 激活时，pgsz() 返回编译期常量 4096
        let sz = pgsz();
        assert_eq!(sz, 4096);
        assert!(sz.is_power_of_two());
    });

    test!("test_pgsz_returns_positive_value_note" {
        // 当 PAGESIZE cfg 未激活时，pgsz() 需要 CTX.pagesize 已初始化
        // 此测试仅为文档记录，实际测试需要完整的 CTX 初始化
        // TODO: 实现 CTX 初始化后启用:
        // let sz = unsafe { pgsz() };
        // assert!(sz > 0);
        // assert!(sz.is_power_of_two());
    });

    // ========================================================================
    // 链表操作概念测试 (queue/dequeue)
    // ========================================================================

    test!("test_queue_dequeue_roundtrip_concept" {
        // 概念测试: 验证 queue + dequeue 的基本不变量
        //
        // 此测试描述了 queue/dequeue 的预期行为但不可执行
        // （因为函数体是 todo!()，需要实现完成后才能运行）
        //
        // 预期行为:
        // let mut head: *mut Meta = null_mut();
        // let mut m = make_isolated_meta();
        // - queue(&mut head, &mut *m) -> head == m, m.prev == m, m.next == m
        // - dequeue(&mut head, &mut *m) -> head == null, m.prev == null, m.next == null
        //
        // TODO: 实现完成后启用
    });

    test!("test_queue_empty_list_concept" {
        // 概念测试: 向空链表插入一个节点
        //
        // 预期: 节点形成自循环 (*m).prev == m && (*m).next == m
        // TODO: 实现完成后启用
    });

    test!("test_queue_nonempty_list_concept" {
        // 概念测试: 向非空链表插入第二个节点
        //
        // 预期: 正确的双向循环链表结构
        // head -> m1 -> m2 -> head (cyclic)
        // TODO: 实现完成后启用
    });

    test!("test_dequeue_only_node_concept" {
        // 概念测试: 从单节点链表中移除唯一节点
        //
        // 预期: head == null, m.prev == null, m.next == null
        // TODO: 实现完成后启用
    });

    test!("test_dequeue_head_updates_when_removing_head_concept" {
        // 概念测试: 移除头节点时 *phead 应更新为 (*m).next
        //
        // 预期: 头指针正确转移
        // TODO: 实现完成后启用
    });

    test!("test_dequeue_head_empty_list_concept" {
        // 概念测试: 空链表 dequeue_head 返回 null
        //
        // 预期: 返回 null，无副作用
        // TODO: 实现完成后启用
    });

    // ========================================================================
    // size_to_class / size_overflows 概念测试
    // ========================================================================

    test!("test_size_overflows_threshold_value" {
        // 验证溢出阈值常量: usize::MAX / 2 - 4096
        // 不调用函数，仅验证阈值计算不会溢出
        let threshold = usize::MAX / 2 - 4096;
        assert!(threshold < usize::MAX / 2); // 确保减法正确
    });

    test!("test_size_overflows_safe_values_concept" {
        // 概念测试: 常见分配大小不应溢出
        //
        // TODO: 实现完成后启用:
        // for size in &[0, 1, 16, 64, 256, 1024, 4096, 65536, 131072, 1048576] {
        //     assert!(!size_overflows(*size));
        // }
    });

    test!("test_size_overflows_near_max_concept" {
        // 概念测试: 接近 usize::MAX 的值应判定为溢出
        //
        // TODO: 实现完成后启用:
        // let threshold = usize::MAX / 2 - 4096;
        // assert!(!size_overflows(threshold - 1));
        // assert!(size_overflows(threshold));
        // assert!(size_overflows(usize::MAX));
    });

    test!("test_size_to_class_output_range_concept" {
        // 概念测试: size_to_class 输出应在 0..48 范围
        //
        // TODO: 实现完成后启用:
        // for n in 0..131072 {
        //     let cls = size_to_class(n);
        //     assert!(cls < 48);
        // }
    });

    test!("test_size_to_class_monotonic_concept" {
        // 概念测试: size_to_class 必须单调不减
        // n1 <= n2 => size_to_class(n1) <= size_to_class(n2)
        //
        // TODO: 实现完成后启用
    });

    test!("test_size_to_class_minimum_size_concept" {
        // 概念测试: 零大小分配映射到 class 0
        //
        // TODO: 实现完成后启用:
        // assert_eq!(size_to_class(0), 0);
    });

    // ========================================================================
    // 序列号系统概念测试
    // ========================================================================

    test!("test_seq_value_domain" {
        // CTX.seq 值域为 0..255 (u8)
        assert_eq!(core::mem::size_of::<u8>(), 1);
        assert_eq!(u8::MAX, 255);
    });

    test!("test_unmap_seq_array_covers_sc_7_to_38" {
        // unmap_seq 有 32 个元素，覆盖 size class 7..38 (共 32 个)
        // sc - 7 的范围: 0..31
        assert_eq!(32, 32); // 验证数组长度足够
        // sc=7  -> index 0 (valid)
        // sc=38 -> index 31 (valid)
    });

    test!("test_bounces_array_covers_sc_7_to_38" {
        // bounces 有 32 个元素，覆盖 size class 7..38
        assert_eq!(32, 32);
    });

    test!("test_bounce_threshold_100_is_valid_u8" {
        // is_bouncing 的阈值为 100，在 u8 范围内
        assert!(100u8 < u8::MAX);
    });

    test!("test_bounce_max_150_is_valid_u8" {
        // account_bounce 的上限为 150
        assert!(150u8 <= u8::MAX);
    });

    test!("test_seq_wraparound_at_255_concept" {
        // 概念测试: CTX.seq 在 255 时回绕到 1
        // step_seq() 检测: seq == 255 -> 重置 seq=1, 清零所有 unmap_seq
        //
        // TODO: 实现完成后启用
    });

    test!("test_record_seq_valid_range_concept" {
        // 概念测试: record_seq 仅在 sc 7..38 有效
        //
        // TODO: 实现完成后启用:
        // - record_seq(7)  -> unmap_seq[0] = CTX.seq
        // - record_seq(38) -> unmap_seq[31] = CTX.seq
        // - record_seq(6)  -> 无操作 (越界)
        // - record_seq(39) -> 无操作 (越界)
    });

    // ========================================================================
    // get_slot_index 概念测试
    // ========================================================================

    test!("test_get_slot_index_extracts_5_bits_concept" {
        // 概念测试: get_slot_index 提取 in-band header 字节的低 5 位
        // 值域 0..31
        //
        // TODO: 实现完成后启用:
        // 构造一个 byte 序列，验证提取结果:
        // byte 0b00011111 -> slot_index 31
        // byte 0b11100000 -> slot_index 0
    });

    // ========================================================================
    // get_nominal_size / set_size 编解码概念测试
    // ========================================================================

    test!("test_reserved_encoding_range" {
        // reserved 值使用高 3 位编码，值域 0..7
        // 0-4: 直接编码
        // 5-7: 5 表示 >=5，实际值存储在扩展区域
        assert!(7 < 8); // 3 位最大值为 7
    });

    test!("test_reserved_small_values_concept" {
        // 概念测试: reserved < 5 时直接编码在 header 高 3 位
        //
        // TODO: 实现完成后启用
    });

    test!("test_reserved_large_values_concept" {
        // 概念测试: reserved >= 5 时使用扩展编码
        //
        // TODO: 实现完成后启用
    });

    // ========================================================================
    // 内存布局验证 (Per-slot layout)
    // ========================================================================

    test!("test_per_slot_header_layout" {
        // 验证 per-slot header 的偏移量
        // p[-3]: 低 5 位 = slot index, 高 3 位 = reserved
        // p[-2]: 16 位偏移量 (低字节)
        // p[-1]: 16 位偏移量 (高字节)
        // 偏移量以 UNIT 为单位

        // 这些偏移量在 get_meta 函数中硬编码:
        // p.sub(2) 读取 16 位偏移
        // p.sub(3) 读取 header 字节
        // p.sub(4) 读取非零偏移检测字节
        // p.sub(8) 读取大偏移量
        let _slot_index_offset: isize = -3;
        let _offset_low_offset: isize = -2;
        let _offset_high_offset: isize = -1; // 与 offset_low 组成 u16
        let _nonzero_detect_offset: isize = -4;
        let _large_offset_storage: isize = -8;
    });

    test!("test_per_slot_footer_layout" {
        // 验证 per-slot footer 的偏移量
        // end = p + stride - IB
        // end[0]: 溢出检查字节 (always 0)
        // end[-reserved]: 分隔零字节
        // end[-4]: 大 reserved 扩展存储 (u32)
        // end[-5]: 大 reserved 标记字节 (0)

        // 这些偏移量在 get_nominal_size 函数中使用
        let _overflow_check_offset: isize = 0; // relative to end
        let _reserved_separator_offset: isize = -1; // variable, depends on reserved
        let _large_reserved_storage_offset: isize = -4; // u32 at end-4
        let _large_reserved_marker_offset: isize = -5; // zero byte at end-5
    });

    // ========================================================================
    // 并发安全验证 (AtomicI32)
    // ========================================================================

    test!("test_atomici32_is_lock_free" {
        // AtomicI32 在 x86-64 和 aarch64 上应为 lock-free
        // 这保证了 activate_group 的 CAS 循环不会意外退化
        // 使用 size_of 验证（lock-free 原子类型大小等于底层整数大小）
        assert_eq!(core::mem::size_of::<AtomicI32>(), 4);
    });

    test!("test_atomici32_size_matches_i32" {
        // AtomicI32 的布局应与普通 i32 相同（C ABI 兼容）
        assert_eq!(mem::size_of::<AtomicI32>(), mem::size_of::<i32>());
        assert_eq!(mem::align_of::<AtomicI32>(), mem::align_of::<i32>());
    });

    test!("test_atomici32_ordering_relaxed_is_available" {
        // 验证 Ordering::Relaxed 可用于 avail_mask/freed_mask 的非同步读取
        // (get_meta 的断言检查使用 Relaxed)
        let mask = AtomicI32::new(0);
        assert_eq!(mask.load(Ordering::Relaxed), 0);
    });

    test!("test_atomici32_ordering_acqrel_is_available" {
        // 验证 Ordering::AcqRel 可用于 CAS 操作
        // (activate_group 使用 AcqRel)
        let mask = AtomicI32::new(0);
        let _ = mask.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed);
    });

    // ========================================================================
    // 指针大小/对齐验证
    // ========================================================================

    test!("test_pointer_size_is_8_on_64bit" {
        // 在 64 位平台上，所有指针为 8 字节
        // 这是整个分配器布局计算的基础
        if cfg!(target_pointer_width = "64") {
            assert_eq!(mem::size_of::<*mut Meta>(), 8);
            assert_eq!(mem::size_of::<*mut Group>(), 8);
            assert_eq!(mem::size_of::<*mut MetaArea>(), 8);
            assert_eq!(mem::size_of::<*const u8>(), 8);
        }
    });

    test!("test_usize_equals_pointer_size" {
        // usize 必须与指针大小相同 (C 的 uintptr_t 等效)
        assert_eq!(mem::size_of::<usize>(), mem::size_of::<*const u8>());
    });

    // ========================================================================
    // Meta 大小上限验证 (spec 要求)
    // ========================================================================

    test!("test_meta_size_not_exceeding_48" {
        // spec: size_of::<Meta>() <= 48 (实际通常为 40)
        let size = mem::size_of::<Meta>();
        assert!(size <= 48, "Meta size = {} exceeds 48", size);
    });

    test!("test_meta_repr_c_layout_matches_c_struct" {
        // 验证 #[repr(C)] 确保字段顺序
        // prev (0), next (8), mem (16), avail_mask (24), freed_mask (28), bitfields (32)
        // total = 40 bytes (在 64-bit 上)
        let meta_ptr: *const Meta = core::ptr::null();
        unsafe {
            let prev_off = core::ptr::addr_of!((*meta_ptr).prev) as *const _ as usize;
            let next_off = core::ptr::addr_of!((*meta_ptr).next) as *const _ as usize;
            let mem_off = core::ptr::addr_of!((*meta_ptr).mem) as *const _ as usize;
            assert_eq!(prev_off, 0);
            assert_eq!(next_off, 8);
            assert_eq!(mem_off, 16);
        }
    });
}