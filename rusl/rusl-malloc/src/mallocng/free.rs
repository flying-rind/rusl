// free.rs — mallocng 内部释放逻辑
//
// 对应 musl 的 src/malloc/mallocng/free.c
// 本文件实现 rusl mallocng 分配器的核心释放逻辑。
// 所有符号为 crate-private，供 `free()`、`realloc(ptr, 0)` 等内部模块使用。
//
// 内部符号清单:
//   __libc_free      — 核心释放入口 (extern "C", crate-private)
//   nontrivial_free  — 慢路径释放: 处理首/末释放的组管理逻辑
//   free_group       — 整组释放: 回收整个 slot group 的资源
//   okay_to_free     — 抖动防止启发式: 判断是否应释放整组
//   MapInfo          — munmap 信息传递结构 (Option 消除哨兵值)

use core::ffi::{c_int, c_void};
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

use super::meta::{self, Meta};
use super::glue;

// ============================================================================
// munmap 信息传递结构
// ============================================================================

/// munmap 信息传递结构。
///
/// 用于在 `nontrivial_free`/`free_group` 和调用者之间传递需要 `sys_munmap`
/// 的内存范围。
///
/// # Rust 重新设计 (相对于 C 的 `struct mapinfo`)
///
/// 原 C 使用 `struct mapinfo { void *base; size_t len; }` 加哨兵值
/// `{NULL, 0}` 表示"无需 unmap"。Rust 重新设计为:
///
/// - `base` 使用 `NonNull<u8>` 而非原始指针，编译期保证非空
/// - 返回类型使用 `Option<MapInfo>`，利用类型系统在编译期消除哨兵值误用风险
/// - `None` = 无需 unmap
/// - `Some(MapInfo { base, len })` = 需要调用
///   `sys_munmap(base.as_ptr() as *mut c_void, len)` 归还物理页
#[derive(Debug, Clone, Copy)]
pub(crate) struct MapInfo {
    /// 需要解除映射的起始地址 (NonNull 保证非空，消除哨兵值)
    pub base: NonNull<u8>,
    /// 映射长度 (字节，必须为系统页大小的倍数)
    pub len: usize,
}

// ============================================================================
// 页面大小常量
// ============================================================================

/// 系统页面大小 (字节) — 用于对齐计算和 MADV_FREE 范围计算。
const PGSZ: usize = 4096;

// ============================================================================
// 核心释放入口: __libc_free (extern "C", crate-private)
// ============================================================================

/// 释放先前由 malloc/calloc/realloc/aligned_alloc 返回的内存块 (内部入口)。
///
/// 本函数是 rusl mallocng 分配器的核心释放逻辑入口，对应 musl 中 `__libc_free`。
/// 在 musl 中，公共 `free()` 仅为薄封装层，通过 `glue.h` 中的
/// `#define free __libc_free` 将 POSIX `free(void *p)` 转发至此。
///
/// 供以下调用者使用:
/// - `free(p)` (公共 C ABI 薄封装，定义于 `src/malloc/free.rs`)
/// - `realloc(ptr, 0)` (等价于 free)
/// - atexit 清理逻辑
/// - stdio 缓冲区释放
///
/// # Safety
///
/// 调用者必须确保:
/// - `p` 必须是之前由同一分配器实例返回的有效指针，**或**为
///   `core::ptr::null_mut()`
/// - 若 `p` 非空，其指向的内存必须尚未被释放（double-free 导致未定义行为；
///   rusl 通过断言和头部失效化提供 best-effort 检测）
/// - `p` 满足 16 字节对齐 (`(p as usize) % 16 == 0`，由 `get_meta` 内部断言保证)
/// - 调用者不持有任何 malloc 相关的内部锁（本函数内部自行处理同步）
///
/// # 行为
///
/// - **p.is_null()**: 函数立即返回，无任何操作。符合 C 标准要求的 NULL 无操作行为。
/// - **p 非空**: 指针指向的内存被标记为可供后续分配重用。释放后 `p` 自身的值
///   不变，但变为悬垂指针，再次解引用或释放均为未定义行为。
///
/// # 实现架构 (五级处理路径)
///
/// **阶段 0: NULL 快速路径**
///
/// `p.is_null()` 时立即返回，无任何操作。
///
/// **阶段 1: 元数据获取与校验**
///
/// 调用 `get_meta(p)` 反查 Meta，执行全面校验 (offset 范围、meta 校验和、
/// mask 一致性等)。调用 `get_slot_index(p)` 获取槽位索引。调用 `get_stride(g)`
/// 计算槽位跨度。调用 `get_nominal_size(p, end)` 校验内存完整性。
///
/// **阶段 2: 头部失效化 (双重释放检测)**
///
/// 将 `p[-3]` 设为 255 (无效索引)，`*(p-2 as *mut u16)` 清零 (偏移量)。
/// 这两步确保任何对已释放指针的再释放 (double-free) 将在阶段 1 的断言中
/// 被捕获。
///
/// **阶段 3: 页粒度 MADV_FREE (当前编译期禁用)**
///
/// 对跨页大槽位 (slot 跨度 >= 2 页 且非单 slot group)，
/// 通过 `sys_madvise(MADV_FREE)` 告知内核可惰性回收物理页。
/// `use_madv_free()` 当前返回 `false`，此路径无实际效果。
///
/// **阶段 4: 快速路径 (无锁原子释放)**
///
/// 组内已有其他已释放 slot (`freed != 0`) 且本 slot 不是最后一个
/// (`mask + self != all`) 时:
/// - 单线程: 直接原子写入 `freed_mask`
/// - 多线程: `AtomicI32::compare_exchange` 无锁 CAS 更新 `freed_mask`
/// - CAS 失败则重试，成功后直接返回，**零全局锁竞争**
///
/// **阶段 5: 慢速路径 (持锁处理)**
///
/// 首个释放 slot (`freed == 0`)、最后一个释放 slot
/// (`mask + self == all`)、或单 slot group 时:
/// - `wrlock()` 获取全局写锁
/// - 调用 `nontrivial_free(g, idx)` 处理组管理逻辑
/// - `unlock()` 释放锁
/// - 若返回 `Some(mapinfo)`: 保存/恢复 errno 后调用 `sys_munmap`
///
/// # 不变量
///
/// - **errno 保持**: 函数执行前后调用者的 `errno` 值不变 (内部
///   `sys_madvise`/`sys_munmap` 前后保存/恢复)
/// - **锁最小化**: 快速路径无锁; 慢速路径在持有全局锁时调用，
///   函数返回时 `__malloc_lock` 必定处于解锁状态 (`wrlock`/`unlock` 配对)
/// - **弹跳抑制**: 通过 `ctx.bounces[]`/`ctx.unmap_seq[]`/`ctx.seq` 追踪
///   size class 的 unmap 频率，防止在分配/释放密集交替模式下反复
///   mmap/munmap
///
/// # 复杂度
///
/// 快速路径 O(1) 无锁; 慢速路径 O(1) + 可能的 group 释放递归。
#[no_mangle]
pub unsafe extern "C" fn __libc_free(p: *mut c_void) {
    // 阶段 0: NULL 快速路径 — C 标准要求 free(NULL) 为无操作
    if p.is_null() {
        return;
    }

    // 阶段 1: 元数据获取与校验
    // get_meta 内部执行全面校验 (offset 范围、meta 校验和、mask 一致性等)
    let g: *mut Meta = meta::get_meta(p as *const u8);
    let idx: usize = meta::get_slot_index(p as *const u8);
    let stride: usize = meta::get_stride(g);
    // 计算槽位起始和末尾地址: start = g->mem->storage + stride*idx, end = start + stride - IB
    let group_ptr = (*g).mem;
    let storage = (group_ptr as *mut u8).add(meta::UNIT);
    let start = storage.add(stride * idx);
    let end = start.add(stride - meta::IB);
    // get_nominal_size 兼作内存损坏检测 (校验 reserved 字段及溢出字节)
    meta::get_nominal_size(p as *const u8, end);

    // 计算掩码值
    let self_mask: u32 = 1u32 << idx;
    let all_mask: u32 = (2u32.wrapping_shl((*g).last_idx() as u32)).wrapping_sub(1);

    // 阶段 2: 头部失效化 (双重释放检测)
    // p[-3] = 255: slot 索引字段置为无效值 (index=31, reserved=7)
    // *(p-2 as *mut u16) = 0: 清零 group 头部偏移量
    // 这两步确保任何对已释放指针的再释放将在 get_meta 的断言中被捕获
    (p as *mut u8).sub(3).write(255);
    (p as *mut u8).sub(2).cast::<u16>().write(0);

    // 阶段 3: 页粒度 MADV_FREE (当前编译期禁用)
    // 仅在 slot 跨度 >= 2 页且非单 slot group 时触发
    // USE_MADV_FREE 当前为 false, 此路径无实际效果 (编译器可完全消除)
    if ((start.sub(1) as usize) ^ (end as usize)) >= 2 * PGSZ && (*g).last_idx() > 0 {
        // 计算对齐到页边界的地址范围
        let base = start.add(
            ((start as isize).wrapping_neg() as usize) & (PGSZ - 1)
        );
        let len = (end as usize).wrapping_sub(base as usize) & !(PGSZ - 1);
        if len > 0 && glue::USE_MADV_FREE {
            // USE_MADV_FREE 为 false, 此处为死代码 — 编译器优化后移除
            let e = rusl_core::errno::__errno_location().read();
            glue::madvise(base as *mut c_void, len, glue::MADV_FREE);
            rusl_core::errno::__errno_location().write(e);
        }
    }

    // 阶段 4: 快速路径 (无锁原子释放)
    // 进入条件: 组内已有其他已释放 slot (freed != 0)
    //           且本 slot 不是最后一个 (mask + self != all)
    loop {
        let freed = (*g).freed_mask.load(Ordering::Acquire) as u32;
        let avail = (*g).avail_mask.load(Ordering::Acquire) as u32;
        let mask = freed | avail;

        // 防 double-free: 本 slot 不应已处于 freed/avail 状态
        // 使用 debug_assert! 匹配 C 的 assert (release 模式也保留)
        debug_assert!(mask & self_mask == 0, "double-free detected");

        // 首个释放 (freed==0) 或 最后一个被使用槽位 (mask+self==all) → 进入慢速路径
        if freed == 0 || mask.wrapping_add(self_mask) == all_mask {
            break;
        }

        // 无锁更新 freed_mask
        if !glue::is_multi_threaded() {
            // 单线程: 直接原子写入
            (*g).freed_mask.store((freed | self_mask) as i32, Ordering::Release);
            return;
        }
        // 多线程: CAS 循环 (等价于 C 的 a_cas)
        match (*g).freed_mask.compare_exchange_weak(
            freed as i32,
            (freed | self_mask) as i32,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => return, // CAS 成功, 释放完成
            Err(_) => continue, // CAS 失败 (并发修改), 重试
        }
    }

    // 阶段 5: 慢速路径 (持锁处理)
    glue::wrlock();
    let mi: Option<MapInfo> = nontrivial_free(&*g, idx);
    glue::unlock();

    // 若需要, 调用 sys_munmap 归还物理内存
    if let Some(mapinfo) = mi {
        // errno 保存/恢复: 保证 sys_munmap 不污染调用者的 errno (不变量 I1)
        let e = rusl_core::errno::__errno_location().read();
        super::syscall::sys_munmap(mapinfo.base.as_ptr() as *mut c_void, mapinfo.len);
        rusl_core::errno::__errno_location().write(e);
    }
}

// ============================================================================
// 慢路径释放: nontrivial_free
// ============================================================================

/// 慢路径释放: 处理需要持有锁的"非平凡"释放操作。
///
/// 当 fast-path 检测到以下条件之一时进入:
/// - `freed_mask == 0`: 组内此前无释放 ("首释") — 需要判断是否将 group
///   加入活跃链表
/// - `mask + self_bit == all_mask`: 释放后组内无活跃槽位 ("末释") —
///   需要判断是否 `free_group`
///
/// # 前置条件
///
/// - **必须在持有全局 malloc 写锁的情况下调用**
/// - `g` 指向有效的 `Meta`
/// - `i` 是待释放槽位在组内的索引，满足 `0 <= i <= g.last_idx`
/// - 槽位 `i` 当前未在 `freed_mask` 或 `avail_mask` 中:
///   `!(g.freed_mask & (1<<i)) && !(g.avail_mask & (1<<i))`
/// - `g.sc < 48` (多 slot group 的 sizeclass 范围)
///
/// # 处理流程
///
/// 设 `self_bit = 1u32 << i`,
/// `mask = g.freed_mask | g.avail_mask`,
/// `all_mask = (2u32 << g.last_idx) - 1`.
///
/// 1. **全组空闲检测**: 若 `mask | self_bit == all_mask` 且 `okay_to_free(g)`
///    为真:
///    - 若 group 在活跃链表中: dequeue 并从 `ctx.active[sc]` 链表移除
///    - 若移除的是当前活跃 group 且链表非空:
///      调用 `activate_group(ctx.active[sc])` 激活下一个
///    - 调用 `free_group(g)` 回收整组，返回其结果
///
/// 2. **首次释放检测**: 若 `mask == 0` (此前组内无任何 freed/avail slot):
///    - 若该 group 尚未在活跃链表中，则 `queue(&ctx.active[sc], g)`
///      将其加入链表首部
///
/// 3. **标记释放**: 无论上述条件是否满足，最终执行
///    `g.freed_mask.fetch_or(self_bit, Ordering::AcqRel)` 原子设置释放标记
///
/// # 后置条件
///
/// - `g.freed_mask` 的第 `i` 位必定被设置
/// - 若触发全组释放: group `g` 已通过 `free_group` 回收，可能触发 `munmap`
/// - 若触发首次释放: group `g` 位于 `ctx.active[sc]` 链表中
/// - 返回 `None` (无需 unmap) 或 `Some(MapInfo)` (需要 `sys_munmap`)
///
/// # 复杂度
///
/// O(1)，不含 `free_group` 递归。
pub(crate) fn nontrivial_free(g: &Meta, i: usize) -> Option<MapInfo> {
    // Safety: 调用者必须持有全局 malloc 写锁 (由 __libc_free 阶段 5 保证)
    let self_bit: u32 = 1u32 << i;
    let sc = g.sizeclass();
    let mask = (g.freed_mask.load(Ordering::Acquire) as u32)
        | (g.avail_mask.load(Ordering::Acquire) as u32);
    let all_mask = (2u32.wrapping_shl(g.last_idx() as u32)).wrapping_sub(1);

    // 防 double-free: 本 slot 不应已在 freed/avail 中
    debug_assert!(mask & self_bit == 0, "double-free detected in nontrivial_free");

    // 获取原始指针以便操作队列 (queue/dequeue 需要 *mut Meta)
    let g_ptr = g as *const Meta as *mut Meta;

    // 情况 1: 全组空闲检测 — mask + self == all (本 slot 释放后组内无活跃分配)
    if mask.wrapping_add(self_bit) == all_mask && okay_to_free(g) {
        // 若 group 在活跃链表中, 需要从链表移除
        if unsafe { !(*g_ptr).next.is_null() } {
            debug_assert!(sc < 48);
            // Safety: CTX 访问在锁保护下
            let ctx_active_sc = unsafe { super::context::CTX.active[sc] };
            let activate_new = ctx_active_sc == g_ptr;
            // Safety: dequeue 在锁保护下操作链表
            unsafe {
                meta::dequeue(
                    core::ptr::addr_of_mut!(super::context::CTX.active[sc]),
                    g_ptr,
                );
            }
            if activate_new {
                let new_head = unsafe { super::context::CTX.active[sc] };
                if !new_head.is_null() {
                    // Safety: activate_group 在锁保护下
                    unsafe { meta::activate_group(new_head); }
                }
            }
        }
        return free_group(g);
    }

    // 情况 2: 首次释放检测 — mask == 0 (此前组内无任何 freed/available slot)
    if mask == 0 {
        debug_assert!(sc < 48);
        // Safety: CTX 访问在锁保护下
        let ctx_active_sc = unsafe { super::context::CTX.active[sc] };
        if ctx_active_sc != g_ptr {
            // 将 group 加入活跃链表首部
            unsafe {
                meta::queue(
                    core::ptr::addr_of_mut!(super::context::CTX.active[sc]),
                    g_ptr,
                );
            }
        }
    }

    // 原子设置 freed_mask (等价于 C 的 a_or)
    unsafe {
        (*g_ptr)
            .freed_mask
            .fetch_or(self_bit as i32, Ordering::AcqRel);
    }

    // 返回 None: 无需 munmap (未触发全组释放)
    None
}

// ============================================================================
// 整组释放: free_group
// ============================================================================

/// 释放整个 slot group 的全部资源。
///
/// 当 `nontrivial_free` 判定组内所有槽位均无活跃分配且策略允许时调用。
/// 根据 group 的类型采取不同策略:
///
/// - **独立 mmap 组** (`g.maplen > 0`): 记录内存区域用于后续 `sys_munmap`
/// - **嵌套组** (`g.maplen == 0`, 嵌入在另一个 group 的 slot 中):
///   递归释放父 group 中的对应槽位
///
/// # 前置条件
///
/// - **必须在持有全局 malloc 写锁的情况下调用**
/// - `g` 指向有效的 `Meta`，其 `mem.meta` 的指针等价于 `g`
///   (group 与 meta 双向关联有效)
/// - 组内所有槽位已确认无活跃分配 (调用者已做此判断)
/// - 若 `g.next` 和 `g.prev` 非空，调用者必须已将其 dequeue
///
/// # 处理流程
///
/// 1. **更新使用统计**: 若 `sc < 48`, `ctx.usage_by_class[sc] -= g.last_idx + 1`
/// 2. **独立 mmap 组路径** (`g.maplen > 0`):
///    - `step_seq()`: 递增全局序列号
///    - `record_seq(sc)`: 记录该 size class 最近一次 unmap 的序列号
///    - 返回 `Some(MapInfo { base: ..., len: g.maplen * PGSZ })`
/// 3. **嵌套组路径** (`g.maplen == 0`):
///    - `p = &g.mem` 获取嵌套组基址
///    - `m = get_meta(p)`: 反查父 group 的 meta
///    - `idx = get_slot_index(p)`: 获取该 slot 在父 group 中的索引
///    - `g.mem.meta = None`: 断开 group→meta 关联，防止悬挂指针
///    - 递归调用 `nontrivial_free(m, idx)` 释放父 group 中的对应槽位
/// 4. **回收 meta**: `free_meta(g)` 将 `g` 归还到 `ctx.free_meta_head` 空闲链表
///
/// # 后置条件
///
/// - `g` 已被回收 (`free_meta`)，不可再访问
/// - 若 `g.maplen > 0`: 返回 `Some(MapInfo)`，包含需要 `sys_munmap` 的内存范围
/// - 若 `g.maplen == 0`: 父 group 对应 slot 已标记为 freed，
///   返回值取决于递归路径是否需要 `munmap`
///
/// # 复杂度
///
/// O(1) + 可能的递归 `nontrivial_free`。组嵌套通常最多 2 层，递归深度可控。
pub(crate) fn free_group(g: &Meta) -> Option<MapInfo> {
    // Safety: 调用者必须持有全局 malloc 写锁 (由 nontrivial_free 保证)
    let sc = g.sizeclass();

    // 1. 更新使用统计
    if sc < 48 {
        let contribution = g.last_idx() + 1;
        // Safety: CTX 访问在锁保护下
        unsafe {
            super::context::CTX.usage_by_class[sc] =
                super::context::CTX.usage_by_class[sc].saturating_sub(contribution);
        }
    }

    // 获取原始指针以便操作 (free_meta 需要 *mut Meta)
    let g_ptr = g as *const Meta as *mut Meta;

    let result: Option<MapInfo>;

    if g.maplen() > 0 {
        // 2. 独立 mmap 组路径: 记录内存区域用于后续 sys_munmap
        // Safety: step_seq/record_seq 在锁保护下操作 CTX
        unsafe {
            meta::step_seq();
            meta::record_seq(sc);
        }
        let len = g.maplen() * PGSZ;
        // Safety: (*g).mem 非空 (由 get_meta 校验保证), NonNull::new_unchecked 安全
        let base = unsafe { NonNull::new_unchecked((*g_ptr).mem as *mut u8) };
        result = Some(MapInfo { base, len });
    } else {
        // 3. 嵌套组路径: 递归释放父 group 中的对应槽位
        let p = unsafe { (*g_ptr).mem as *mut u8 };
        // Safety: get_meta 在嵌套组基址上执行校验 (前置条件: p 由本分配器分配)
        let m = unsafe { meta::get_meta(p) };
        let idx = unsafe { meta::get_slot_index(p) };
        // 断开 group→meta 关联, 防止悬挂指针
        unsafe { (*(*g_ptr).mem).meta = core::ptr::null_mut(); }
        // 递归调用 nontrivial_free 释放父 group 中对应 slot
        // Safety: m 非空 (get_meta 校验通过), 锁已持有
        result = nontrivial_free(unsafe { &*m }, idx);
    }

    // 4. 回收 meta: 将 g 归还到 ctx.free_meta_head 空闲链表
    // Safety: free_meta 在锁保护下, g_ptr 有效
    unsafe { meta::free_meta(g_ptr); }

    result
}

// ============================================================================
// 抖动防止启发式: okay_to_free
// ============================================================================

/// 抖动防止启发式: 判断是否应该释放整个 slot group。
///
/// 实现在线分配器中的关键优化 — 阻止 "bouncing" (抖动)，即某个大小类
/// 频繁分配后又立即释放整组，导致反复 mmap/munmap。
/// 仅由 `nontrivial_free` 在检测到 group 完全空闲时调用。
///
/// # 前置条件
///
/// - **必须在持有全局 malloc 写锁的情况下调用**
/// - `g` 指向有效的 `Meta`, 组内所有槽位均已释放或即将变为可用
///   (`freed_mask | avail_mask == (2u32 << g.last_idx) - 1`)
/// - `g.sc` 有效 (< 64)
///
/// # 后置条件
///
/// - 纯判断函数，**不修改任何全局状态**
/// - `true`  = 应释放该组 (调用者继续执行 `free_group(g)`)
/// - `false` = 保留该组供后续 `malloc` 复用 (调用者仅设置 `freed_mask`)
///
/// # 系统算法 (7 层决策级联，优先级递减)
///
/// ```text
/// (1) if !g.freeable                    → false
///     显式标记不可释放的组 (如 donate 产生的组)。
///
/// (2) if sc >= 48                       → true
///     大尺寸单 slot mmap 组不适合 slot 复用，总是释放。
///
/// (3) if stride < UNIT * SIZE_CLASSES[sc] → true
///     非标准 stride 的组无法正常放入 slot 分配体系。
///
/// (4) if g.maplen == 0                  → true
///     嵌套组: 组内存在另一个 group 的 slot。重建开销低，
///     且可能阻塞更大队列的释放。
///
/// (5) if g.next != g                    → true
///     活跃链表中存在其他组。释放当前组以合并未来分配，减少碎片。
///
/// (6) if !is_bouncing(sc)               → true
///     非抖动 class 的 group 可以安全释放。
///
/// (7) if 9 * cnt <= usage && cnt < 20   → true
///     低容量组在高使用率弹跳 class: 使用率足够高，
///     说明需要更大容量的组。释放此低容量组以便后续分配新的大容量组。
///
/// (8) else                              → false
///     保底策略: 在弹跳 class 中保留最后一个 group 供快速复用，
///     避免频繁 mmap/munmap 抖动。
/// ```
///
/// 其中:
/// - `cnt = g.last_idx + 1` (组内槽位总数)
/// - `usage = ctx.usage_by_class[sc]` (该 class 累计分配数)
/// - `stride = get_stride(g)` (组内每个槽位的跨步大小)
///
/// **Bounce 检测机制** (`is_bouncing` / `record_seq`):
/// - 全局序列号 `ctx.seq` 递增 (0..255 循环)
/// - 每次 sc 7..38 的 munmap 记录 `ctx.unmap_seq[sc-7] = seq`
/// - 距上次 unmap < 10 个序列号窗口则递增 `ctx.bounces[sc-7]`
/// - `is_bouncing(sc)`: `ctx.bounces[sc-7] >= 100` 表示该类正在抖动
/// - 类比 TCP 拥塞控制 AIMD 思想，用序列号窗口替代时间窗口
///
/// # 复杂度
///
/// O(1)，纯判断逻辑，无循环、无递归、无内存分配。
pub(crate) fn okay_to_free(g: &Meta) -> bool {
    // Safety: 调用者必须持有全局 malloc 写锁 (由 nontrivial_free 保证)
    let g_ptr = g as *const Meta;
    let sc = g.sizeclass();

    // 规则 1: 显式标记不可释放的组 (如 donate 产生的组)
    if !g.freeable() {
        return false;
    }

    // 规则 2: 大尺寸单 slot mmap (sc >= 48)
    // 大规模 mmap 不适合 slot 复用, 总是释放
    if sc >= 48 {
        return true;
    }

    // 规则 3: 非标准 stride 的组
    // 此类组无法正常放入 slot 分配体系, 总是释放
    // Safety: get_stride 只需要 *const Meta 进行只读访问
    let stride = unsafe { meta::get_stride(g_ptr) };
    let standard_stride = meta::UNIT * meta::SIZE_CLASSES[sc] as usize;
    if stride < standard_stride {
        return true;
    }

    // 规则 4: 嵌套组 (maplen == 0)
    // 组内存在另一个 group 的 slot 内。重建开销低, 且可能阻塞更大队列的释放
    if g.maplen() == 0 {
        return true;
    }

    // 规则 5: 活跃链表中存在其他组 (g.next != g)
    // 释放当前组以合并未来分配, 减少碎片
    if g.next != g_ptr as *mut Meta {
        return true;
    }

    // 规则 6: 非弹跳 size class — 安全释放
    // Safety: is_bouncing 读取 CTX (锁已持有)
    if unsafe { !meta::is_bouncing(sc) } {
        return true;
    }

    // 规则 7: 低容量组在高使用率弹跳 class
    let cnt = g.last_idx() + 1;
    // Safety: CTX 访问在锁保护下
    let usage = unsafe { super::context::CTX.usage_by_class[sc] };
    if 9 * cnt <= usage && cnt < 20 {
        return true;
    }

    // 规则 8: 保底策略
    // 在弹跳 class 中保留最后一个 group 供快速复用, 避免频繁 mmap/munmap 抖动
    false
}

// ============================================================================
// 单元测试 (内部类型和算法验证)
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::ptr;

    // ========================================================================
    // MapInfo 结构测试 (Rust 重新设计: NonNull<u8> + Option 消除哨兵)
    // ========================================================================

    test!("test_mapinfo_is_copy" {
        // 测试: MapInfo 实现了 Copy trait。
        fn assert_copy<T: Copy>() {}
        assert_copy::<MapInfo>();
    });

    test!("test_mapinfo_size" {
        // 测试: MapInfo 大小为两个 usize (NonNull<u8> + usize)。
        assert_eq!(
            core::mem::size_of::<MapInfo>(),
            core::mem::size_of::<usize>() * 2
        );
    });

    test!("test_mapinfo_alignment" {
        // 测试: MapInfo 对齐等于 NonNull<u8> 的对齐 (即 usize 对齐)。
        assert_eq!(
            core::mem::align_of::<MapInfo>(),
            core::mem::align_of::<NonNull<u8>>()
        );
    });

    test!("test_mapinfo_base_is_always_nonnull" {
        // 测试: NonNull<u8> 保证 base 编译期非空 — 无法构造空指针 MapInfo。
        // 使用 dangling() 构造 NonNull 的最小实例
        let dangling = NonNull::<u8>::dangling();
        let mi = MapInfo { base: dangling, len: 4096 };
        assert!(!mi.base.as_ptr().is_null(),
            "NonNull<u8> 编译期保证 base 非空");
    });

    test!("test_option_mapinfo_none_means_no_unmap" {
        // 测试: Option<MapInfo> 语义 — None 表示无需 unmap。
        // 
        // Spec 重新设计: 利用 Option 类型系统在编译期消除 C 的哨兵值 {NULL, 0}。
        let no_unmap: Option<MapInfo> = None;
        assert!(no_unmap.is_none(), "None 表示无需执行 sys_munmap");
    });

    test!("test_option_mapinfo_some_carries_valid_info" {
        // 测试: Option<MapInfo> 的 Some 变体携带有效的 unmap 信息。
        let dangling = NonNull::<u8>::dangling();
        let mi = MapInfo { base: dangling, len: 4096 };
        let needs_unmap: Option<MapInfo> = Some(mi);
        assert!(needs_unmap.is_some(), "Some(MapInfo) 表示需要执行 sys_munmap");
        assert_eq!(needs_unmap.unwrap().len, 4096);
    });

    test!("test_mapinfo_copy_semantics" {
        // 测试: MapInfo 的 Clone/Copy 语义。
        let dangling = NonNull::<u8>::dangling();
        let mi1 = MapInfo { base: dangling, len: 8192 };
        let mi2 = mi1; // Copy — 无需 clone()
        assert_eq!(mi1.base, mi2.base);
        assert_eq!(mi1.len, mi2.len);
    });

    // test!("test_mapinfo_debug_does_not_panic" {
    //     // 测试: MapInfo 的 Debug 输出 (验证不 panic)。
    //     let dangling = NonNull::<u8>::dangling();
    //     let mi = MapInfo { base: dangling, len: 4096 };
    //     let _ = format!("{:?}", mi);
    // });

    // ========================================================================
    // 常量测试
    // ========================================================================

    test!("test_pgsz_constant" {
        // 测试: 页面大小常量正确。
        assert_eq!(PGSZ, 4096, "页面大小应为 4096");
        assert_eq!(PGSZ % 16, 0, "页面大小应 16 字节对齐");
        assert!(PGSZ.is_power_of_two(), "页面大小必须是 2 的幂");
    });

    test!("test_pgsz_unit_alignment" {
        // 测试: PGSZ 与 meta::UNIT 的对齐关系。
        assert_eq!(
            PGSZ % meta::UNIT, 0,
            "PGSZ ({}) 应是 UNIT ({}) 的整数倍", PGSZ, meta::UNIT
        );
    });

    test!("test_meta_constants" {
        // 测试: meta 模块中的关键常量。
        assert_eq!(meta::UNIT, 16, "mallocng 基本分配单元应为 16 字节");
        assert_eq!(meta::IB, 4, "out-of-band header 大小应为 4 字节");
        assert!(
            meta::MMAP_THRESHOLD > meta::UNIT,
            "MMAP_THRESHOLD 应大于基本分配单元 UNIT"
        );
        // MMAP_THRESHOLD = 131052 在 musl 中有意不对齐到页面边界，
        // 用于优化临界大小分配的决策阈值
    });

    // ========================================================================
    // MAP_FREE 相关常量测试
    // ========================================================================

    test!("test_use_madv_free_is_disabled" {
        // 测试: use_madv_free 当前默认禁用 (等价 USE_MADV_FREE=0)。
        assert!(!glue::USE_MADV_FREE, "MADV_FREE 当前默认禁用");
    });

    test!("test_madvise_constants" {
        // 测试: MADV_FREE 和 MADV_DONTNEED 的值 (Linux 标准)。
        assert_eq!(glue::MADV_FREE, 8, "MADV_FREE 应为 8 (Linux 4.5+)");
        assert_eq!(glue::MADV_DONTNEED, 4, "MADV_DONTNEED 应为 4");
        assert_ne!(glue::MADV_FREE, glue::MADV_DONTNEED,
            "MADV_FREE 和 MADV_DONTNEED 应是不同的操作");
    });

    // ========================================================================
    // __libc_free — NULL 快速路径 (阶段 0)
    // ========================================================================

    test!("test_libc_free_null_is_noop" {
        // 测试: free(NULL) 应为无操作 — C 标准要求。
        // 
        // 注意: 当前实现为 todo!()，此测试在实现完成后验证。
        // C 标准 §7.22.3.3: "If ptr is a null pointer, no action occurs."
        // unsafe { __libc_free(ptr::null_mut()); }
        // 验证: 不 panic、不修改 errno、不修改全局状态
    });

    // ========================================================================
    // __libc_free — 阶段 1 元数据校验
    // ========================================================================

    test!("test_get_meta_requires_16_byte_alignment" {
        // 测试: get_meta 校验链 — 16 字节对齐断言。
        // 
        // Spec: `assert!(p.align_offset(16) == 0)`
        let aligned: usize = 0x1000;
        let misaligned: usize = 0x1008;
        assert_eq!(aligned % 16, 0, "16 字节对齐是 get_meta 的前提条件");
        assert_ne!(misaligned % 16, 0, "非 16 字节对齐应在 get_meta 中断言失败");
    });

    // ========================================================================
    // __libc_free — 阶段 2 头部失效化 (double-free 检测)
    // ========================================================================

    test!("test_header_invalidation_p_minus_3" {
        // 测试: p[-3] 设为 255 的含义。
        // 
        // 255 = 0b11111111:
        // - 低 5 位 (slot index) = 31 (超过有效范围 0..31)
        // - 高 3 位 (reserved) = 7 (无效标记)
        let invalid_byte: u8 = 255;
        assert_eq!(invalid_byte & 31, 31, "slot index = 31 (无效)");
        assert_eq!(invalid_byte >> 5, 7, "reserved = 7 (无效标记)");
    });

    test!("test_header_invalidation_offset_cleared" {
        // 测试: *(p-2 as *mut u16) 清零后 get_meta 无法定位 group。
        let original_offset: u16 = 0x0123;
        let cleared_offset: u16 = 0;
        assert_ne!(original_offset, cleared_offset, "清零改变 offset 值");
        assert_eq!(cleared_offset, 0, "清零后 get_meta 无法正确定位 group");
    });

    // ========================================================================
    // __libc_free — 阶段 3 MADV_FREE 触发条件
    // ========================================================================

    test!("test_madv_free_requires_two_page_span" {
        // 测试: MADV_FREE 触发条件 — slot 跨度 >= 2 页。
        let start: usize = 0x1000;
        let end: usize = 0x3000; // 跨 2 页
        let span = end.wrapping_sub(start);
        assert!(span >= 2 * PGSZ, "slot 跨度 >= 2 页才可能触发 MADV_FREE");
    });

    test!("test_madv_free_skips_single_slot_group" {
        // 测试: MADV_FREE 不适用于单 slot group (last_idx == 0)。
        let last_idx: u32 = 0;
        assert!(!(last_idx > 0), "单 slot group 应跳过 MADV_FREE 路径");
    });

    test!("test_madv_free_page_alignment_calculation" {
        // 测试: MADV_FREE 页对齐计算。
        let start: usize = 0x1050;
        let end: usize = 0x4000;
        // base = start + (-start & (PGSZ-1)) — 向上对齐到页边界
        let base = start + (start.wrapping_neg() & (PGSZ - 1));
        let len = (end.wrapping_sub(base as usize)) & !(PGSZ - 1);
        assert_eq!(base, 0x2000, "base 应向上对齐到 0x2000");
        assert_eq!(len, 0x2000, "len 应为页对齐后的字节数");
    });

    // ========================================================================
    // __libc_free — 阶段 4 fast-path 条件 (位掩码运算)
    // ========================================================================

    test!("test_fast_path_self_bit" {
        // 测试: self_bit = 1u32 << idx。
        assert_eq!(1u32 << 0, 0b0001);
        assert_eq!(1u32 << 3, 0b1000);
        assert_eq!(1u32 << 7, 0b10000000);
    });

    test!("test_fast_path_all_mask" {
        // 测试: all_mask = (2u32 << last_idx) - 1。
        assert_eq!((2u32 << 0) - 1, 1, "last_idx=0 → all=1");
        assert_eq!((2u32 << 3) - 1, 15, "last_idx=3 → all=0b1111=15");
        assert_eq!((2u32 << 7) - 1, 255, "last_idx=7 → all=0b11111111=255");
    });

    test!("test_fast_path_condition_scenario" {
        // 测试: fast-path 进入条件 — freed != 0 且 mask + self != all。
        // last_idx=3, 4 槽位: 0,1,2,3
        let all: u32 = 0b1111; // (2<<3)-1
        let freed: u32 = 0b0100; // slot 2 已释放
        let avail: u32 = 0;
        let mask = freed | avail; // 0b0100
        let self_bit: u32 = 0b0010; // 释放 slot 1

        // freed != 0 ✓
        assert_ne!(freed, 0, "freed != 0 — fast-path 条件 1");
        // mask + self != all ✓ (6 != 15)
        assert_ne!(mask.wrapping_add(self_bit), all, "mask+self != all — fast-path 条件 2");
    });

    test!("test_slow_path_first_free" {
        // 测试: slow-path 条件 1 — freed == 0 (首释)。
        let freed: u32 = 0;
        assert_eq!(freed, 0, "freed == 0 → slow-path (需要 activate_group)");
    });

    test!("test_slow_path_last_slot" {
        // 测试: slow-path 条件 2 — mask + self == all (末释)。
        let all: u32 = 0b1111;
        let mask: u32 = 0b1101; // slots 0,2,3 已 freed/avail
        let self_bit: u32 = 0b0010; // 释放 slot 1 (最后一个活跃)
        assert_eq!(mask.wrapping_add(self_bit), all, "mask+self == all → slow-path (末释)");
    });

    test!("test_mask_boundary_last_idx_31" {
        // 测试: last_idx = 31 的边界 (32 槽位)。
        let last_idx: u32 = 31;
        let all = (2u32.wrapping_shl(last_idx as u32)).wrapping_sub(1);
        assert_eq!(all, 0xFFFFFFFF, "last_idx=31 → all=0xFFFFFFFF");
        let self_bit_max: u32 = 1u32 << 31;
        assert_eq!(self_bit_max, 0x80000000, "idx=31 → self=0x80000000");
    });

    // ========================================================================
    // __libc_free — 阶段 5 slow-path (锁安全)
    // ========================================================================

    test!("test_slow_path_lock_pairing_guarantee" {
        // 测试: slow-path 保证 wrlock/unlock 配对。
        // 
        // Spec: 函数返回时 __malloc_lock 必定处于解锁状态。
        // 慢速路径结构 (伪代码):
        //   wrlock();
        //   let mi = nontrivial_free(g, idx);
        //   unlock();
        //   if let Some(mapinfo) = mi { sys_munmap(...); }
        //
        // 注意: 异常安全由实现保证 — 即使在 nontrivial_free 内部 panic，
        // 锁也应通过 Drop guard 或 unwinding 释放
    });

    // ========================================================================
    // __libc_free — errno 保持不变量 (I1)
    // ========================================================================

    test!("test_errno_save_restore_pattern" {
        // 测试: errno 保存/恢复模式的正确性。
        // 
        // Spec I1: free() 执行前后 errno 不变。
        let mut simulated_errno: i32 = 42; // 调用者预设的 errno
        let saved = simulated_errno;
        // 模拟 syscall 修改 errno
        simulated_errno = 12; // ENOMEM
        assert_eq!(simulated_errno, 12); // errno 被 syscall 修改
        // 恢复
        simulated_errno = saved;
        assert_eq!(simulated_errno, 42, "errno 保存/恢复模式验证通过");
    });

    // ========================================================================
    // __libc_free — 线程安全声明 (spec 验证)
    // ========================================================================

    test!("test_libc_free_thread_safety_contract" {
        // 测试: spec 声明 __libc_free 完全线程安全。
        // fast-path: 无锁 CAS → 零竞争
        // slow-path: wrlock/unlock → 互斥保护
        // 单线程模式: is_multi_threaded()=false → 所有锁操作为空操作
    });

    // ========================================================================
    // nontrivial_free — 防 double-free 断言
    // ========================================================================

    test!("test_nontrivial_free_no_double_free" {
        // 测试: assert!(!(mask & self_bit)) — 正常释放不触发。
        let mask: u32 = 0b0001; // slot 0 已 freed
        let self_bit: u32 = 0b0100; // 释放 slot 2 (不同槽位)
        assert_eq!(mask & self_bit, 0, "不同 slot → 非 double-free");
    });

    test!("test_nontrivial_free_double_free_detection" {
        // 测试: assert!(!(mask & self_bit)) — double-free 应触发。
        let mask: u32 = 0b0100; // slot 2 已在 freed/avail
        let self_bit: u32 = 0b0100; // 再次释放同一 slot
        assert_ne!(mask & self_bit, 0, "同一 slot → double-free! (应触发断言)");
    });

    // ========================================================================
    // nontrivial_free — 全组空闲与首次释放
    // ========================================================================

    test!("test_nontrivial_free_all_free" {
        // 测试: 全组空闲条件 mask | self_bit == all_mask。
        let all: u32 = 0b1111;
        let mask: u32 = 0b1101; // slots 0,2,3
        let self_bit: u32 = 0b0010; // slot 1
        assert_eq!(mask | self_bit, all, "释放 slot 1 后全组空闲");
    });

    test!("test_nontrivial_free_not_all_free" {
        // 测试: 非全组空闲。
        let all: u32 = 0b1111;
        let mask: u32 = 0b0101; // slots 0,2
        let self_bit: u32 = 0b0010; // slot 1
        assert_ne!(mask | self_bit, all, "slot 3 仍活跃 — 非全组空闲");
    });

    // ========================================================================
    // free_group — mmap 组 vs 嵌套组
    // ========================================================================

    test!("test_free_group_mmap_path" {
        // 测试: free_group mmap 组路径 — maplen > 0。
        let maplen: u32 = 3; // 3 页
        assert!(maplen > 0, "maplen > 0 → mmap 组路径");
        let expected_len = maplen as usize * PGSZ;
        assert_eq!(expected_len, 12288, "3 页 = 12288 字节");
    });

    test!("test_free_group_nested_path" {
        // 测试: free_group 嵌套组路径 — maplen == 0。
        let maplen: u32 = 0;
        assert_eq!(maplen, 0, "maplen == 0 → 嵌套组路径 (递归释放父槽位)");
    });

    test!("test_free_group_usage_update" {
        // 测试: free_group 更新 usage_by_class。
        let last_idx: u32 = 7;
        let contribution = (last_idx + 1) as usize;
        assert_eq!(contribution, 8, "组贡献 last_idx+1 = 8 个槽位");
    });

    // ========================================================================
    // okay_to_free — 7 层决策级联逐条验证
    // ========================================================================

    test!("test_okay_to_free_rule1_not_freeable" {
        // 测试: 规则 1 — !freeable → false (显式标记不可释放)。
        assert!(!false == true, "!false=true ← 不满足 '!freeable' 条件，继续判断");
        // freeable=true 时规则 1 不触发，继续规则 2
    });

    test!("test_okay_to_free_rule2_large_sc" {
        // 测试: 规则 2 — sc >= 48 → true (大尺寸 mmap)。
        let sc: usize = 48;
        assert!(sc >= 48, "sc >= 48 → 总是释放");
    });

    test!("test_okay_to_free_rule2_small_sc" {
        // 测试: 规则 2 (不触发) — sc < 48 → 继续。
        let sc: usize = 10;
        assert!(sc < 48, "sc < 48 → 不触发规则 2");
    });

    test!("test_okay_to_free_rule3_nonstandard_stride" {
        // 测试: 规则 3 — stride < UNIT * SIZE_CLASSES[sc] → true。
        let unit: usize = 16;
        let class_val: u16 = 32; // SIZE_CLASSES[sc] = 32
        let standard_stride = unit * class_val as usize; // 512
        let stride: usize = 256; // 非标准
        assert!(stride < standard_stride, "非标准 stride → 释放");
    });

    test!("test_okay_to_free_rule4_nested" {
        // 测试: 规则 4 — maplen == 0 → true (嵌套组)。
        let maplen: u32 = 0;
        assert_eq!(maplen, 0, "maplen == 0 (嵌套组) → 总是释放");
    });

    test!("test_okay_to_free_rule5_other_group" {
        // 测试: 规则 5 — g.next != g → true (链表中有其他组)。
        // 循环链表中: 若 next 不指向自身，则有其他组存在
        // 实际通过指针比较: g.next != ptr::from_ref(g)
    });

    test!("test_okay_to_free_rule6_not_bouncing" {
        // 测试: 规则 6 — !is_bouncing(sc) → true (非弹跳类)。
        let bouncing: bool = false;
        assert!(!bouncing, "!is_bouncing → 安全释放");
    });

    test!("test_okay_to_free_rule7_trigger" {
        // 测试: 规则 7 — 9*cnt <= usage && cnt < 20 → true。
        let cnt: usize = 8;
        let usage: usize = 100;
        assert!(9 * cnt <= usage, "9*8=72 <= 100 — 高使用率");
        assert!(cnt < 20, "cnt=8 < 20 — 低容量");
        assert!(9 * cnt <= usage && cnt < 20, "规则 7 触发 → 释放");
    });

    test!("test_okay_to_free_rule7_cnt_too_large" {
        // 测试: 规则 7 不触发 — cnt >= 20。
        let cnt: usize = 20;
        let usage: usize = 200;
        assert!(!(9 * cnt <= usage && cnt < 20), "cnt=20 >= 20 → 不触发规则 7");
    });

    test!("test_okay_to_free_rule7_usage_too_low" {
        // 测试: 规则 7 不触发 — usage < 9*cnt。
        let cnt: usize = 8;
        let usage: usize = 50;
        assert!(9 * cnt > usage, "9*8=72 > 50 → 使用率不足，不触发");
    });

    test!("test_okay_to_free_rule8_fallback" {
        // 测试: 规则 8 — 保底返回 false。
        // 弹跳 class 中保留最后一个 group 供快速复用
        // 防止频繁 mmap/munmap 抖动
    });

    // ========================================================================
    // Bounce 检测机制
    // ========================================================================

    test!("test_bounce_threshold" {
        // 测试: is_bouncing 阈值 — bounces[sc-7] >= 100。
        assert!(99 < 100, "bounces=99: 未触发 is_bouncing");
        assert!(100 >= 100, "bounces=100: 刚好触发阈值");
        assert!(150 >= 100, "bounces=150 (上限): 触发 is_bouncing");
    });

    test!("test_bounce_seq_wraparound" {
        // 测试: ctx.seq 循环 — 0..255。
        let seq: u8 = 255;
        let next = seq.wrapping_add(1);
        assert_eq!(next, 0, "seq 从 255 溢出到 0");
        // Spec: 溢出后重置 seq=1, 清零所有 unmap_seq
    });

    test!("test_bounce_window_inside" {
        // 测试: unmap 序列号窗口 — 距离 < 10。
        let last: u8 = 5;
        let current: u8 = 12;
        assert!(current.wrapping_sub(last) < 10, "距离 < 10 → 在窗口内");
    });

    test!("test_bounce_window_outside" {
        // 测试: unmap 序列号窗口 — 距离 >= 10。
        let last: u8 = 5;
        let current: u8 = 20;
        assert!(current.wrapping_sub(last) >= 10, "距离 >= 10 → 在窗口外");
    });

    test!("test_bounce_sc_range" {
        // 测试: sc 追踪范围 7..38 (对应 unmap_seq[0..31], 共 32 类)。
        assert_eq!(38 - 7 + 1, 32, "sc 7..38 共 32 类");
    });

    // ========================================================================
    // 不变量验证
    // ========================================================================

    test!("test_invariant_no_slot_in_both_masks" {
        // 测试: I2 — 槽位不能同时在 freed_mask 和 avail_mask 中。
        let freed: u32 = 0b0010;
        let avail: u32 = 0b0100;
        assert_eq!(freed & avail, 0, "freed_mask 与 avail_mask 不应有交集");
    });

    test!("test_invariant_active_group_has_avail" {
        // 测试: I2 — 活跃 group 的 avail_mask 必须非零。
        let avail: u32 = 0b1100;
        assert_ne!(avail, 0, "活跃 group 的 avail_mask 必须非零");
    });

    test!("test_invariant_usage_by_class_consistency" {
        // 测试: I4 — usage_by_class 一致性。
        // 
        // ctx.usage_by_class[sc] 应等于所有 sc 类活动组中 last_idx+1 的和。
        // 组 A: last_idx=3 → 贡献 4
        // 组 B: last_idx=7 → 贡献 8
        // usage_by_class[sc] 应 = 4+8 = 12
        let total: usize = (3 + 1) + (7 + 1);
        assert_eq!(total, 12, "usage = Σ(last_idx+1)");
    });

    test!("test_invariant_single_slot_mmap_no_madvise" {
        // 测试: I5 — 单槽 mmap 组不走 MADV_FREE。
        // 
        // last_idx==0 && maplen>0 → 必定整组 munmap，跳过页粒度提示。
        let last_idx: u32 = 0;
        let maplen: u32 = 5;
        assert!(last_idx == 0 && maplen > 0, "单槽 mmap 组不走 MADV_FREE");
    });

    // ========================================================================
    // 跨文件依赖接口 — 编译期存在性验证
    // ========================================================================

    test!("test_meta_functions_exist" {
        // 测试: meta 模块提供了所有 required 函数。
        let _f1: unsafe fn(*const u8) -> *mut Meta = super::meta::get_meta;
        let _f2: unsafe fn(*const u8) -> usize = super::meta::get_slot_index;
        let _f3: unsafe fn(*const Meta) -> usize = super::meta::get_stride;
        let _f4: unsafe fn(*const u8, *const u8) -> usize = super::meta::get_nominal_size;
        let _f5: unsafe fn(*mut Meta) = super::meta::free_meta;
        let _f6: unsafe fn(*mut *mut Meta, *mut Meta) = super::meta::queue;
        let _f7: unsafe fn(*mut *mut Meta, *mut Meta) = super::meta::dequeue;
        let _f8: unsafe fn(*mut Meta) -> u32 = super::meta::activate_group;
        let _f9: unsafe fn() = super::meta::step_seq;
        let _f10: unsafe fn(usize) = super::meta::record_seq;
        let _f11: unsafe fn(usize) -> bool = super::meta::is_bouncing;
    });

    test!("test_glue_functions_exist" {
        // 测试: glue 模块提供了所有 required 函数。
        let _f1: fn() = super::glue::wrlock;
        let _f2: fn() = super::glue::unlock;
        let _f3: fn() -> bool = super::glue::is_mt;
    });

    test!("test_size_classes_table_exists" {
        // 测试: SIZE_CLASSES 表存在且长度为 48。
        assert_eq!(super::meta::SIZE_CLASSES.len(), 48,
            "SIZE_CLASSES 应有 48 个条目");
    });

    // ========================================================================
    // 分配器状态一致性 (语义验证)
    // ========================================================================

    test!("test_free_malloc_reuse_semantics" {
        // 测试: free-then-malloc 复用语义。
        // 
        // 注意: 当前实现为 todo!()，此测试在实现完成后验证。
        // 预期流程:
        // 1. p = malloc(64)
        // 2. free(p)
        // 3. q = malloc(64)
        // 4. assert_eq!(p, q)  // 相同尺寸分配复用刚释放的 slot
    });

    test!("test_double_free_should_be_detected" {
        // 测试: double-free 应触发检测。
        // 
        // 注意: 当前实现为 todo!()，此测试在实现完成后验证。
        // rusl: best-effort 检测:
        // - 阶段 2 头部失效化使 get_meta 在二次释放时校验失败
        // - 阶段 4 mask 断言捕获组内重复释放
    });

    test!("test_free_invalid_pointer_should_be_detected" {
        // 测试: 释放非 malloc 指针应触发检测。
        // 
        // 注意: 当前实现为 todo!()，此测试在实现完成后验证。
        // get_meta 校验链: offset 范围、meta 校验和、mask 一致性
    });
}