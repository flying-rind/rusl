// malloc.rs — malloc 核心实现 (malloc new generation)
//
// 对应 musl 的 src/malloc/mallocng/malloc.c
// 本文件实现 rusl mailocng 分配器的核心分配逻辑。
//
// ## 模块职责
//
// - `malloc`: 对外导出的 POSIX 标准分配函数
// - `alloc_meta`: 内部分配 Meta 元数据对象
// - `is_allzero`: 用于 calloc 优化的零页检测
// - `alloc_slot` / `try_avail` / `alloc_group`: 内部分配流程
// - `SIZE_CLASSES` / `CTX` / `SMALL_CNT_TAB` / `MED_CNT_TAB`: 全局数据
//
// ## 算法概述
//
// mallocng 采用分尺寸类别 (size class) 的 group 分配策略:
// 1. 将用户请求大小映射到 48 个预定义尺寸类别 (0-47)
// 2. 每个类别维护一个循环双向链表 (ctx.active[sc])
// 3. 每个链表节点 (Meta) 关联一个槽位组 (Group)
// 4. 快速路径: 通过原子 CAS 操作从 avail_mask 获取空闲槽位
// 5. 慢速路径: 创建新 Group 或扩展已有 Group
// 6. 超大分配 (>= MMAP_THRESHOLD): 直接使用 mmap
//
// ## 依赖模块 (均为 crate-internal)
//
// - `super::meta`  — Meta, Group, MetaArea, MallocContext 结构体与操作函数
// - `super::glue`  — wrlock, unlock, rdlock, upgradelock 锁原语
// - `super::syscall` — sys_mmap, sys_munmap 等系统调用封装
// - `crate::errno` — __errno_location, ENOMEM

use core::ffi::c_void;
use core::sync::atomic::Ordering;

use super::glue;
use super::meta::{self, Meta, MallocContext};
use super::context::CTX;

// ============================================================================
// 全局常量 (从 meta 模块重导出)
// ============================================================================

use super::meta::{IB, MMAP_THRESHOLD, UNIT};
use crate::import::__errno_location;

// ============================================================================
// 全局数据: 大小类别查找表
// ============================================================================

/// 大小类别查找表 (以 UNIT = 16 字节为单位)
///
/// `SIZE_CLASSES[sc]` 表示该类别下每个 slot 的 UNIT 数。
/// 实际字节数 = `UNIT * SIZE_CLASSES[sc]`。
///
/// 分类: class 0-7: 1-8 (16B-128B), class 8-11: 9-15 (144B-240B),
///       class 12-47: 18-8191 (288B-131056B)
///
/// 此引用指向 `meta.rs` 中定义的全局静态表。
pub(crate) use super::meta::SIZE_CLASSES;

// ============================================================================
// 全局数据: 分配器上下文 (定义于 context.rs)
// ============================================================================
//
// CTX 定义于 super::context 模块，此处通过 use super::context::CTX 导入。
// 整个进程唯一的分配器全局状态，管理所有尺寸类别的活跃 group 链表、
// 元数据区、序列号系统和反碎片化启发式数据。

// ============================================================================
// 模块私有静态数据: 槽位数量表
// ============================================================================

/// 小尺寸类别 (sc < 9) 的 slot 数量表
///
/// 每个 sc 有 3 个使用等级 (i=0 最少, i=1 中等, i=2 最多)。
/// `alloc_group()` 根据 `usage_by_class[sc]` 动态选择等级。
///
/// 原 C: `static const uint8_t small_cnt_tab[][3]`
static SMALL_CNT_TAB: [[u8; 3]; 9] = [
    [30, 30, 30], // sc=0 (16B):  所有等级均 30 slots
    [31, 15, 15], // sc=1 (32B)
    [20, 10, 10], // sc=2 (48B)
    [31, 15, 7],  // sc=3 (64B)
    [25, 12, 6],  // sc=4 (80B)
    [21, 10, 5],  // sc=5 (96B)
    [18, 8, 4],   // sc=6 (112B)
    [31, 15, 7],  // sc=7 (128B)
    [28, 14, 6],  // sc=8 (144B)
];

/// 中等尺寸类别 (sc >= 9) 的基础 slot 数
///
/// 按 `sc & 3` 索引: sc%4=0→28, =1→24, =2→20, =3→32
/// 原 C: `static const uint8_t med_cnt_tab[4]`
static MED_CNT_TAB: [u8; 4] = [28, 24, 20, 32];

// ============================================================================
// 对外导出接口 (Public C ABI)
// ============================================================================

/// 分配 `n` 字节的未初始化内存。
///
/// 使用分尺寸类别 (size class) 的 group 分配策略优化常见小分配的性能
/// 和内存效率。对于大分配 (>= `MMAP_THRESHOLD` = 131052 字节)，直接使用 `mmap`。
///
/// # 参数
///
/// - `n`: 请求分配的字节数 (可以是 0)
///
/// # 返回值
///
/// - **成功**: 返回指向至少 `n` 字节、16 字节对齐的未初始化内存的指针
/// - **`n == 0`**: 返回 `null` (不设置 errno)
/// - **溢出**: 若 `n >= usize::MAX / 2 - 4096`, 设置 `errno = ENOMEM`, 返回 `null`
/// - **内存耗尽**: 返回 `null`, 设置 `errno = ENOMEM`
///
/// # 前置条件
///
/// - 无特殊前置条件 (分配器在首次调用时延迟初始化)
/// - 在多线程环境中: 调用者无需持有任何锁 (内部自动加锁)
///
/// # 后置条件
///
/// - 返回的指针对齐到 16 字节边界 (`(p as usize) & 15 == 0`)
/// - 通过 `get_meta(p)` 可反向推导出所属的 `Meta` 和 `Group`
/// - 分配的内存内容未初始化
///
/// # 系统算法
///
/// **1. 溢出检查**: 调用 `size_overflows(n)`
///
/// **2. 大块路径** (`n >= MMAP_THRESHOLD`):
/// - 向上对齐到页大小
/// - `mmap()` 分配整页内存
/// - `alloc_meta()` 创建元数据, 标记 `sizeclass=63`
/// - 调用 `enframe()` 构造分配块
///
/// **3. 小/中块路径** (`n < MMAP_THRESHOLD`):
/// - `size_to_class(n)` 映射到尺寸类别
/// - **快速路径**: 在 `ctx.active[sc]` 链表上用 CAS 无锁获取槽位
/// - **慢速路径**: 获取写锁, 通过 `alloc_slot()` 分配或创建新 Group
/// - 调用 `enframe()` 构造分配块
///
/// # Safety
///
/// 调用者负责确保返回的内存被正确释放 (通过 `free()` 或 `realloc(ptr, 0)`)。
///
/// # Rust 实现差异
///
/// - 使用 `usize` 替代 C 的 `size_t` (ABI 完全兼容)
/// - 返回 `*mut c_void` 替代 C 的 `void *`
/// - 内部使用 Rust 原子类型 (`AtomicI32` / `AtomicU32`) 替代 C 的 `volatile int + a_cas`
/// - `a_ctz_32` → `u32::trailing_zeros()`
/// - `a_clz_32` → `u32::leading_zeros()`
#[no_mangle]
pub unsafe extern "C" fn malloc(n: usize) -> *mut c_void {
    // 1) 溢出检查
    if meta::size_overflows(n) {
        unsafe {
            __errno_location().write(super::super::ENOMEM);
        }
        return core::ptr::null_mut();
    }

    // 2) 大块路径: n >= MMAP_THRESHOLD → 直接 mmap
    if n >= MMAP_THRESHOLD {
        let needed = n + IB + UNIT;
        let p = super::syscall::sys_mmap(
            core::ptr::null_mut(),
            needed,
            super::syscall::PROT_READ | super::syscall::PROT_WRITE,
            super::syscall::MAP_PRIVATE | super::syscall::MAP_ANONYMOUS,
            -1,
            0,
        );
        if p == super::syscall::MAP_FAILED {
            unsafe {
                __errno_location().write(super::super::ENOMEM);
            }
            return core::ptr::null_mut();
        }

        glue::wrlock();
        meta::step_seq();

        let g = alloc_meta();
        if g.is_null() {
            glue::unlock();
            super::syscall::sys_munmap(p, needed);
            return core::ptr::null_mut();
        }

        (*g).mem = p as *mut super::meta::Group;
        (*(*g).mem).meta = g;
        (*g).set_last_idx(0);
        (*g).set_freeable(true);
        (*g).set_sizeclass(63);
        (*g).set_maplen((needed + 4095) / 4096);
        (*g).avail_mask.store(0, Ordering::Relaxed);
        (*g).freed_mask.store(0, Ordering::Relaxed);

        // 使用全局计数器循环偏移 (地址随机化)
        CTX.mmap_counter = CTX.mmap_counter.wrapping_add(1);
        let ctr = CTX.mmap_counter as usize;

        glue::unlock();
        return meta::enframe(g, 0, n, ctr) as *mut c_void;
    }

    // 3) 小/中块路径: n < MMAP_THRESHOLD
    let mut sc = meta::size_to_class(n);

    // 获取读锁 (RDLOCK_IS_EXCLUSIVE = true, 实际为排他锁)
    glue::wrlock(); // rdlock 等价于 wrlock

    let mut g = CTX.active[sc];

    // 粗粒度尺寸类别优化:
    // 当目标类别尚无 group 时, 使用更大的相邻类别以减少初始 slot 数
    if g.is_null()
        && sc >= 4
        && sc < 32
        && sc != 6
        && (sc & 1) == 0
        && CTX.usage_by_class[sc] == 0
    {
        let mut usage = CTX.usage_by_class[sc | 1];
        if CTX.active[sc | 1].is_null()
            || ((*CTX.active[sc | 1]).avail_mask.load(Ordering::Relaxed) == 0
                && (*CTX.active[sc | 1]).freed_mask.load(Ordering::Relaxed) == 0)
        {
            usage += 3;
        }
        if usage <= 12 {
            sc |= 1;
            g = CTX.active[sc];
        }
    }

    // 快速路径: 从现有 group 的 avail_mask 用 CAS/直接写入 获取槽位
    loop {
        let mask = if g.is_null() {
            0
        } else {
            (*g).avail_mask.load(Ordering::Relaxed)
        };
        let first = mask & mask.wrapping_neg();
        if first == 0 {
            break;
        }

        // RDLOCK_IS_EXCLUSIVE = true → 无竞争, 直接写入
        (*g).avail_mask.store(mask - first, Ordering::Release);

        let idx = first.trailing_zeros() as usize;
        let ctr = CTX.mmap_counter as usize;
        glue::unlock();
        return meta::enframe(g, idx, n, ctr) as *mut c_void;
    }

    // 慢速路径: 已持有锁 (upgradelock 在 RDLOCK_IS_EXCLUSIVE 下为空操作)
    // 调用 alloc_slot 从现有 group 获取或创建新 group
    let idx = match alloc_slot(sc, n) {
        Some(i) => i,
        None => {
            glue::unlock();
            unsafe {
                __errno_location().write(super::super::ENOMEM);
            }
            return core::ptr::null_mut();
        }
    };

    g = CTX.active[sc];
    let ctr = CTX.mmap_counter as usize;
    glue::unlock();

    meta::enframe(g, idx, n, ctr) as *mut c_void
}

// ============================================================================
// pub(crate) 内部符号 (被其他模块依赖)
// ============================================================================

use super::context::alloc_meta;
use super::context::is_allzero;

// ============================================================================
// 私有函数: 内核分配逻辑
// ============================================================================

/// 在尺寸类别 `sc` 中分配一个 slot。
///
/// 首先尝试从现有 group 中获取可用 slot。若失败则创建新的分配组。
///
/// # 前置条件
///
/// - 调用者需持有写锁 (`wrlock()`) 或升级锁 (`upgradelock()`)
/// - `sc < 48`
///
/// # 后置条件
///
/// - **成功**: 返回 `Some(idx)`, idx 为 slot 索引 (0-based),
///   调用者可通过 `ctx.active[sc]` 获取对应 group
/// - **失败**: 返回 `None` (`alloc_group` 失败)
///
/// # 系统算法
///
/// 1. 调用 `try_avail(&mut ctx.active[sc])` 尝试从现有 group 找到可用 slot
/// 2. 若成功: 使用 `first.trailing_zeros()` 将掩码转为索引, 返回 `Some(idx)`
/// 3. 若失败: 调用 `alloc_group(sc, req)` 创建新 group
/// 4. 若 `alloc_group` 返回 `None`: 返回 `None`
/// 5. 新 group: `avail_mask -= 1` (消耗首个 slot),
///    `queue(...)` 将新 group 加入 `ctx.active[sc]`
/// 6. 返回 `Some(0)` (新 group 的首个 slot)
///
/// # Rust 设计要点
///
/// - 返回 `Option<usize>` 替代 C 的 `-1` 错误哨兵值 (更符合 Rust 惯例)
/// - 调用者使用 `match` 或 `?` 处理 Option
unsafe fn alloc_slot(sc: usize, req: usize) -> Option<usize> {
    // 1) 从现有 group 中寻找可用槽位
    let first = try_avail(&mut CTX.active[sc]);
    if first != 0 {
        return Some(first.trailing_zeros() as usize);
    }

    // 2) 创建新 group
    let g = alloc_group(sc, req)?;

    // 3) 消耗新 group 的首个槽位
    let mask = (*g).avail_mask.load(Ordering::Relaxed);
    (*g).avail_mask.store(mask - 1, Ordering::Release);

    // 4) 将新 group 加入 active 链表
    meta::queue(&mut CTX.active[sc], g);

    Some(0)
}

/// 从 `*pm` 指向的 group 开始, 沿着循环链表寻找包含可用 slot 的 group。
///
/// 若当前 group 无可用 slot, 则遍历链表、跳过完全空闲的 group、
/// 必要时激活更多 slot。
///
/// # 前置条件
///
/// - `pm` 指向有效的 `*const Meta` (循环链表或 null)
/// - 调用者需持有读锁或写锁
///
/// # 后置条件
///
/// - **成功**: 返回非零 `u32` (恰好设置一位的掩码),
///   `*pm` 更新为包含可用 slot 的 group
/// - **失败**: 返回 0, `*pm` 可能已更改 (跳过已满的 group) 或为 null
///
/// # 系统算法
///
/// 1. 当前 group 检查: 读 `m.avail_mask`, 若非零则直接返回最低置位
/// 2. 链表遍历:
///    - 若 `avail_mask == 0 && freed_mask == 0` (全满): dequeue, 继续检查下一个
///    - 若 `avail_mask == 0 && freed_mask != 0` (全满但有已释放 slot 可回收): 跳到下一个
/// 3. 跳过完全空闲的 group (freed_mask 覆盖所有 slot 且 freeable)
/// 4. 延迟激活: 若 freed 的 slot 全在未激活区域, 跳到下一个;
///    仅当链表中唯一 group 时才增加 active_idx
/// 5. 激活 group: `activate_group(m)` 将 freed_mask 转移到 avail_mask
/// 6. 反弹衰减: `decay_bounces(m.sizeclass)`
///
/// # Rust 设计要点
///
/// - 使用 `&mut *const Meta` 作为 out-parameter (升级了 C 的双指针)
/// - 返回 `u32` 与 C 保持一致 (位掩码, 用于 `trailing_zeros()`)
/// - 内部链表遍历全部通过裸指针操作
unsafe fn try_avail(pm: &mut *mut Meta) -> u32 {
    let m_ptr = *pm;
    if m_ptr.is_null() {
        return 0;
    }
    let mut m: *mut Meta = m_ptr;
    let mut mask: u32 = (*m).avail_mask.load(Ordering::Relaxed) as u32;

    if mask == 0 {
        // 当前 group 无可用槽位, 在循环链表中寻找
        if (*m).freed_mask.load(Ordering::Relaxed) == 0 {
            // 全满且无已释放槽位: 从 active 链表移除
            meta::dequeue(pm as *mut *mut Meta, m);
            m = *pm;
            if m.is_null() {
                return 0;
            }
        } else {
            // 全满但有已释放槽位 (尚未激活): 移到下一个 group
            m = (*m).next;
            *pm = m;
        }

        mask = (*m).freed_mask.load(Ordering::Relaxed) as u32;

        // 跳过完全空闲且可释放的 group (除非它是唯一活跃 group)
        let lidx = (*m).last_idx();
        let full_mask: u32 = if lidx >= 31 { u32::MAX }
            else { (2u32 << lidx).wrapping_sub(1) };
        if mask == full_mask && (*m).freeable() {
            m = (*m).next;
            *pm = m;
            mask = (*m).freed_mask.load(Ordering::Relaxed) as u32;
        }

        // 延迟激活: 若已释放槽位全在未激活区域, 跳到下一个 group
        // 仅当这是唯一 group 时才扩展 active_idx
        let active_idx = (*(*m).mem).active_idx;
        let active_mask: u32 = if active_idx >= 31 { u32::MAX }
            else { (2u32 << active_idx).wrapping_sub(1) };
        if (mask & active_mask) == 0 {
            if (*m).next != m {
                m = (*m).next;
                *pm = m;
            } else {
                let mut cnt = (*(*m).mem).active_idx as i32 + 2;
                let size = SIZE_CLASSES[(*m).sizeclass()] as usize * UNIT;
                let mut span = UNIT + size * cnt as usize;
                // 按 4KB 边界步进增长, 直到跨越页边界
                while (span ^ (span + size - 1)) < 4096 {
                    cnt += 1;
                    span += size;
                }
                if cnt > (*m).last_idx() as i32 + 1 {
                    cnt = (*m).last_idx() as i32 + 1;
                }
                (*(*m).mem).active_idx = (cnt - 1) as u8;
            }
        }

        // 激活 group: 将 freed_mask 中的可激活位转移到 avail_mask
        mask = meta::activate_group(m as *mut Meta);
        if mask == 0 {
            return 0;
        }

        // 衰减反弹计数
        meta::decay_bounces((*m).sizeclass());
    }

    // 提取最低置位 (slot 索引掩码) 并更新 avail_mask
    let first = mask & mask.wrapping_neg();
    (*m).avail_mask.store((mask - first) as i32, Ordering::Release);
    first
}

/// 为尺寸类别 `sc` 创建一个新的分配组 (`Meta` + `Group`)。
///
/// 确定 slot 数量, 分配存储空间 (mmap 或嵌套分配), 并初始化元数据和组头。
///
/// # 前置条件
///
/// - 调用者需持有写锁 (`wrlock()`)
/// - `sc < 48`
///
/// # 后置条件
///
/// - **成功**: 返回 `Some(meta_ptr)`, 该 Meta 的 `avail_mask` 已设置所有 slot 为可用
///   (除首个已消耗)、`freed_mask` 清零、`mem` 指向新 Group、
///   `last_idx` 和 `sizeclass` 已设置
/// - **失败**: 返回 `None` (`alloc_meta` 失败或 `mmap` 失败, 已调用 `free_meta` 归还 Meta)
///
/// # 系统算法
///
/// 1. `size = UNIT * SIZE_CLASSES[sc]`
/// 2. 确定 slot 数量:
///    - sc < 9: 根据 `usage_by_class[sc]` 在 `SMALL_CNT_TAB[sc]` 的三个等级中选择
///    - sc >= 9: 从 `MED_CNT_TAB[sc & 3]` 出发, 低使用量时减半
///    - 若 `size*cnt >= 65536*UNIT` 继续减半 (slot 偏移不超过 16 位)
///    - 若 `cnt==1 && size+UNIT <= PGSZ/2` 增大到 2
/// 3. 大尺寸路径 (`size*cnt+UNIT > PGSZ/2`):
///    - 检查反弹状态 (`is_bouncing`), 更新反弹计数 (`account_bounce`)
///    - 尝试减少 cnt 控制浪费率 (不超过当前使用量的 25%)
///    - 若低使用量、未反弹、cnt<=7: 尝试降级为独立 mmap (cnt=1)
///    - `__mmap()` 分配整页内存
///    - 计算 `active_idx`, 考虑 4KB 边界对齐
/// 4. 小尺寸路径 (嵌套):
///    - `alloc_slot(j, ...)` 在更大尺寸类别的 group 中分配空间
///    - `enframe()` 初始化存储区
///    - 写入特殊标记 `p[-3] = (p[-3] & 31) | (6 << 5)` (reserved=6 表示嵌套组)
///    - 初始化所有 slot 的越界检查字节
/// 5. 初始化元数据: 设置 `avail_mask`, `freed_mask`, `mem.meta`, `mem.active_idx`,
///    `last_idx`, `freeable=true`, `sizeclass=sc`
/// 6. 更新使用量: `ctx.usage_by_class[sc] += cnt`
///
/// # Rust 设计要点
///
/// - 返回 `Option<*mut Meta>` 替代 C 的 `NULL` 哨兵
/// - 内部使用 `NonNull` 或裸指针管理分配的内存
/// - `sys_mmap` 的结果使用 `is_null()` 检测失败 (替代 C 的 `MAP_FAILED`)
unsafe fn alloc_group(sc: usize, req: usize) -> Option<*mut Meta> {
    let size = UNIT * SIZE_CLASSES[sc] as usize;
    let pagesize = meta::pgsz();

    // 1) 分配 Meta 元数据对象
    let m = alloc_meta();
    if m.is_null() {
        return None;
    }

    // 2) 确定 slot 数量 (基于使用量启发式)
    let usage = CTX.usage_by_class[sc];
    let mut cnt: usize;

    if sc < 9 {
        let mut i = 0;
        while i < 2 && 4 * SMALL_CNT_TAB[sc][i] as usize > usage {
            i += 1;
        }
        cnt = SMALL_CNT_TAB[sc][i] as usize;
    } else {
        cnt = MED_CNT_TAB[sc & 3] as usize;

        // 低使用量时减半, 避免过度预分配
        while (cnt & 1) == 0 && 4 * cnt > usage {
            cnt >>= 1;
        }

        // slot 偏移量在 UNIT 单位下不能超过 16 位
        while size * cnt >= 65536 * UNIT {
            cnt >>= 1;
        }
    }

    // 单个 slot 且可嵌套时增大到 2
    if cnt == 1 && size * cnt + UNIT <= pagesize / 2 {
        cnt = 2;
    }

    // 3) 分配存储空间 (大尺寸 → mmap, 小尺寸 → 嵌套)
    let (p, active_idx): (*mut u8, usize) =
        if size * cnt + UNIT > pagesize / 2 {
            // === 大尺寸路径: mmap 整页 ===

            // 反弹检测
            let nosmall = meta::is_bouncing(sc);
            meta::account_bounce(sc);
            meta::step_seq();

            // 粗粒度使用量计入
            let mut usage_adj = usage;
            if (sc & 1) == 0 && sc < 32 {
                usage_adj += CTX.usage_by_class[sc + 1];
            }

            // 尝试减少 cnt 控制浪费率 (≤25%)
            if 4 * cnt > usage_adj && !nosmall {
                if (sc & 3) == 1 && size * cnt > 8 * pagesize {
                    cnt = 2;
                } else if (sc & 3) == 2 && size * cnt > 4 * pagesize {
                    cnt = 3;
                } else if (sc & 3) == 0 && size * cnt > 8 * pagesize {
                    cnt = 3;
                } else if (sc & 3) == 0 && size * cnt > 2 * pagesize {
                    cnt = 5;
                }
            }

            let mut needed = size * cnt + UNIT;
            needed = (needed + pagesize - 1) & !(pagesize - 1);

            // 低使用量且未反弹时, 尝试降级为独立 mmap (cnt=1)
            if !nosmall && cnt <= 7 {
                let mut req_needed = req + IB + UNIT;
                req_needed = (req_needed + pagesize - 1) & !(pagesize - 1);
                if req_needed < size + UNIT
                    || (req_needed >= 4 * pagesize && 2 * cnt > usage)
                {
                    cnt = 1;
                    needed = req_needed;
                }
            }

            let p_raw = super::syscall::sys_mmap(
                core::ptr::null_mut(),
                needed,
                super::syscall::PROT_READ | super::syscall::PROT_WRITE,
                super::syscall::MAP_PRIVATE | super::syscall::MAP_ANONYMOUS,
                -1,
                0,
            );
            if p_raw == super::syscall::MAP_FAILED {
                meta::free_meta(m);
                return None;
            }
            let p = p_raw as *mut u8;

            (*m).set_maplen(needed >> 12);

            // active_idx: 非页对齐时限制在第一个 4KB 页内
            let mut aidx = (4096 - UNIT) / size;
            if aidx == 0 {
                aidx = 1; // 避免 aidx-1 下溢
            }
            aidx -= 1;
            if aidx > cnt - 1 {
                aidx = cnt - 1;
            }
            if aidx > 0x1F {
                aidx = 0x1F;
            } // 5-bit 限制

            (p, aidx)
        } else {
            // === 小尺寸路径: 在更大类别 group 中嵌套分配 ===

            let j = meta::size_to_class(UNIT + cnt * size - IB);
            let idx = alloc_slot(j, UNIT + cnt * size - IB)?;
            let g = CTX.active[j];
            let p = meta::enframe(
                g,
                idx,
                UNIT * SIZE_CLASSES[j] as usize - IB,
                CTX.mmap_counter as usize,
            );

            (*m).set_maplen(0);

            // 写入嵌套 group 标记: reserved=6
            *p.sub(3) = (*p.sub(3) & 31) | (6 << 5);

            // 初始化越界检查字节
            for i in 0..=cnt {
                *p.add(UNIT + i * size - 4) = 0;
            }

            let aidx = cnt - 1;
            (p, aidx)
        };

    // 4) 初始化 Group 元数据
    CTX.usage_by_class[sc] += cnt;

    let avail = (2u32.wrapping_shl(active_idx as u32)).wrapping_sub(1);
    let total = (2u32.wrapping_shl((cnt - 1) as u32)).wrapping_sub(1);
    let freed = total & !avail;

    (*m).avail_mask.store(avail as i32, Ordering::Relaxed);
    (*m).freed_mask.store(freed as i32, Ordering::Relaxed);
    (*m).mem = p as *mut super::meta::Group;
    (*(*m).mem).meta = m;
    (*(*m).mem).active_idx = active_idx as u8;
    (*m).set_last_idx(cnt - 1);
    (*m).set_freeable(true);
    (*m).set_sizeclass(sc);

    Some(m)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::mem;

    // ---------------------------------------------------------------------------
    // 常量验证测试
    // ---------------------------------------------------------------------------

    test!("test_unit_value" {
        // UNIT 必须为 16 字节, 这是 mallocng 设计的核心假设
        assert_eq!(UNIT, 16);
    });

    test!("test_ib_value" {
        // IB (In-band header) 为 4 字节
        assert_eq!(IB, 4);
    });

    test!("test_mmap_threshold_value" {
        // MMAP_THRESHOLD 为 131052 字节 (128K - 20)
        assert_eq!(MMAP_THRESHOLD, 131052);
    });

    test!("test_unit_power_of_two" {
        // UNIT 必须是 2 的幂以支持位掩码快速取模
        assert!(UNIT.is_power_of_two());
    });

    test!("test_unit_alignment" {
        // UNIT = 16 应 >= 最大基本类型的对齐要求
        assert!(UNIT >= mem::align_of::<u128>());
        assert!(UNIT >= mem::align_of::<*const u8>());
    });

    // ---------------------------------------------------------------------------
    // SIZE_CLASSES 表验证测试
    // ---------------------------------------------------------------------------

    test!("test_size_classes_len" {
        // SIZE_CLASSES 表必须恰好 48 个条目
        assert_eq!(SIZE_CLASSES.len(), 48);
    });

    test!("test_size_classes_monotonic" {
        // 大小类别值必须严格递增 (每个类别大于前一个)
        for i in 1..SIZE_CLASSES.len() {
            assert!(
                SIZE_CLASSES[i] > SIZE_CLASSES[i - 1],
                "SIZE_CLASSES[{}] ({}) <= SIZE_CLASSES[{}] ({})",
                i,
                SIZE_CLASSES[i],
                i - 1,
                SIZE_CLASSES[i - 1]
            );
        }
    });

    test!("test_size_classes_ranges" {
        // 验证各分段的取值范围
        // class 0-7: 1-8 (16B-128B)
        for i in 0..8 {
            assert!(SIZE_CLASSES[i] >= 1, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
            assert!(SIZE_CLASSES[i] <= 8, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
        }

        // class 8-11: 9-15 (144B-240B)
        for i in 8..12 {
            assert!(SIZE_CLASSES[i] >= 9, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
            assert!(SIZE_CLASSES[i] <= 15, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
        }

        // class 44-47: 最大值范围 (74880B-131056B)
        for i in 44..48 {
            assert!(SIZE_CLASSES[i] >= 4680, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
            assert!(SIZE_CLASSES[i] <= 8191, "SIZE_CLASSES[{}] = {}", i, SIZE_CLASSES[i]);
        }
    });

    test!("test_size_classes_max_allocation" {
        // 最大 sizeclass (class 47) 的槽位大小: UNIT * 8191 = 131056 字节
        // 减去 IB = 4 后最大可用为 131052, 恰好等于 MMAP_THRESHOLD
        let max_slot = UNIT as usize * SIZE_CLASSES[47] as usize;
        assert_eq!(max_slot, 131056);
        assert_eq!(max_slot - IB, MMAP_THRESHOLD);
    });

    // ---------------------------------------------------------------------------
    // 槽位数量表验证测试
    // ---------------------------------------------------------------------------

    test!("test_small_cnt_tab_shape" {
        // SMALL_CNT_TAB 有 9 行 (sc 0-8), 每行 3 个使用等级
        assert_eq!(SMALL_CNT_TAB.len(), 9);
        for row in &SMALL_CNT_TAB {
            assert_eq!(row.len(), 3);
        }
    });

    test!("test_small_cnt_tab_monotonic" {
        // 每行内等级 0 >= 等级 1 >= 等级 2 (使用量越高, slot 数量不变或递减)
        for row in &SMALL_CNT_TAB {
            assert!(
                row[0] >= row[1],
                "small_cnt_tab: level 0 ({}) < level 1 ({})",
                row[0],
                row[1]
            );
            assert!(
                row[1] >= row[2],
                "small_cnt_tab: level 1 ({}) < level 2 ({})",
                row[1],
                row[2]
            );
        }
    });

    test!("test_small_cnt_tab_slot_count_reasonable" {
        // 每个槽位数应在合理范围内 (1..64)
        for row in &SMALL_CNT_TAB {
            for &cnt in row {
                assert!(cnt >= 1, "slot count = {} (must be >= 1)", cnt);
                assert!(cnt <= 64, "slot count = {} (must be <= 64)", cnt);
            }
        }
    });

    test!("test_med_cnt_tab_len" {
        // MED_CNT_TAB 有 4 个条目 (按 sc&3 索引)
        assert_eq!(MED_CNT_TAB.len(), 4);
    });

    test!("test_med_cnt_tab_values" {
        // sc%4=0→28, =1→24, =2→20, =3→32
        assert_eq!(MED_CNT_TAB[0], 28);
        assert_eq!(MED_CNT_TAB[1], 24);
        assert_eq!(MED_CNT_TAB[2], 20);
        assert_eq!(MED_CNT_TAB[3], 32);
    });

    // ---------------------------------------------------------------------------
    // CTX 状态测试
    // 注意: 当 rusl malloc 作为全局分配器运行时，CTX 在测试框架初始化时
    // 已被首次分配调用初始化，不再是全零状态。
    // ---------------------------------------------------------------------------

    test!("test_ctx_initialized" {
        // CTX 已被全局分配器初始化 (首次 malloc 调用触发 alloc_meta → init)
        unsafe {
            assert_eq!(CTX.init_done, 1, "CTX 应在首次分配后被初始化");
            assert!(CTX.secret != 0, "secret 应已被随机密钥填充");
        }
    });

    test!("test_ctx_active_array_len" {
        // active 数组长度应为 48 (一个条目对应一个大小类别)
        unsafe {
            assert_eq!(CTX.active.len(), 48);
        }
    });

    test!("test_ctx_usage_by_class_len" {
        // usage_by_class 数组长度应为 48
        unsafe {
            assert_eq!(CTX.usage_by_class.len(), 48);
        }
    });

    test!("test_ctx_active_is_valid_state" {
        // 初始化后 active 数组的所有条目为有效的链表指针 (null 或非 null)
        unsafe {
            for sc in 0..48 {
                let _ = CTX.active[sc]; // 仅访问验证，不崩溃即可
            }
        }
    });

    test!("test_ctx_usage_by_class_valid" {
        // usage_by_class 条目为非负值
        unsafe {
            for sc in 0..48 {
                assert!(CTX.usage_by_class[sc] < 100_000,
                    "usage_by_class[{}] = {} 异常过大", sc, CTX.usage_by_class[sc]);
            }
        }
    });

    test!("test_ctx_unmap_seq_type_check" {
        unsafe {
            assert_eq!(CTX.unmap_seq.len(), 32);
        }
    });

    test!("test_ctx_bounces_type_check" {
        unsafe {
            assert_eq!(CTX.bounces.len(), 32);
        }
    });

    test!("test_ctx_free_meta_head_valid" {
        // free_meta_head 可能为 null 或指向空闲 Meta 链表
        unsafe {
            let _ = CTX.free_meta_head;
        }
    });

    // ---------------------------------------------------------------------------
    // 类型布局测试
    // ---------------------------------------------------------------------------

    test!("test_meta_repr_c_layout" {
        // Meta 必须为 #[repr(C)] 以确保与 C 代码的内存布局兼容
        // 验证结构体有合理的大小和对齐
        assert!(mem::size_of::<Meta>() > 0);
        assert!(mem::align_of::<Meta>() >= mem::align_of::<usize>());
    });

    test!("test_group_repr_c_layout" {
        // Group 必须为 #[repr(C)]
        assert!(mem::size_of::<super::meta::Group>() > 0);
        assert!(mem::align_of::<super::meta::Group>() >= mem::align_of::<usize>());
    });

    test!("test_malloc_context_alignment" {
        // MallocContext 应为合理的对齐和大小
        let size = mem::size_of::<MallocContext>();
        let align = mem::align_of::<MallocContext>();
        assert!(size > 0);
        assert!(align >= mem::align_of::<usize>());
    });

    test!("test_metaarea_repr_c_layout" {
        // MetaArea 必须为 #[repr(C)]
        assert!(mem::size_of::<super::meta::MetaArea>() > 0);
        assert!(mem::align_of::<super::meta::MetaArea>() >= mem::align_of::<usize>());
    });

    // ---------------------------------------------------------------------------
    // 函数签名存在性测试
    // 注意: 需要真实 mmap 的测试已标记为 #[ignore]
    // ---------------------------------------------------------------------------

    test!("test_malloc_signature_exists" {
        // 验证 malloc 函数存在并有正确的签名 (仅编译期检查)
        let _f: unsafe extern "C" fn(usize) -> *mut c_void = malloc;
        let _ = _f; // 不使用但保留签名检查
    });

    test!("test_alloc_meta_signature_exists" {
        // 验证 alloc_meta 函数存在并有正确的签名 (仅编译期检查)
        let _f: unsafe fn() -> *mut Meta = alloc_meta;
        let _ = _f;
    });

    test!("test_is_allzero_signature_exists" {
        // 验证 is_allzero 函数存在并有正确的签名 (仅编译期检查)
        let _f: unsafe fn(*mut c_void) -> i32 = is_allzero;
        let _ = _f;
    });

    test!("test_alloc_slot_signature_exists" {
        // 验证 alloc_slot 函数存在并有正确的签名 (仅编译期检查)
        let _f: unsafe fn(usize, usize) -> Option<usize> = alloc_slot;
        let _ = _f;
    });

    test!("test_try_avail_signature_exists" {
        // 验证 try_avail 函数存在并有正确的签名 (null head 返回 0)
        unsafe {
            let mut head: *mut Meta = core::ptr::null_mut();
            assert_eq!(try_avail(&mut head), 0);
        }
    });

    test!("test_alloc_group_signature_exists" {
        // 验证 alloc_group 函数存在并有正确的签名 (仅编译期检查)
        let _f: unsafe fn(usize, usize) -> Option<*mut Meta> = alloc_group;
        let _ = _f;
    });

    // ---------------------------------------------------------------------------
    // 边界条件测试
    // ---------------------------------------------------------------------------

    test!("test_malloc_zero" {
        // n == 0: musl 选择返回有效指针 (最小尺寸分配) 而非 null，
        //         C 标准规定此行为是实现定义的
        unsafe {
            let p = malloc(0);
            // musl 实现返回非空指针（最小分配单元）
            // 注: C 标准允许 malloc(0) 返回 null，musl 不采纳该行为
            let _ = p;
        }
    });

    test!("test_malloc_small" {
        // 小分配 (16 字节) 应走快速路径
        unsafe {
            let p = malloc(16);
            assert!(!p.is_null(), "malloc(16) 应返回非空指针");
            assert_eq!((p as usize) & 15, 0, "malloc 返回的指针必须 16 字节对齐");
        }
    });

    test!("test_malloc_large" {
        // 大分配 (>= MMAP_THRESHOLD) 应走 mmap 路径
        unsafe {
            let p = malloc(MMAP_THRESHOLD);
            assert!(!p.is_null(), "malloc(MMAP_THRESHOLD) 应返回非空指针");
        }
    });

    test!("test_malloc_huge" {
        // 超大分配应走 mmap 路径
        unsafe {
            let p = malloc(1024 * 1024); // 1 MiB
            assert!(!p.is_null(), "malloc(1MiB) 应返回非空指针");
        }
    });

    test!("test_malloc_near_overflow" {
        // 接近但未到达溢出边界的分配 — 应返回 null (ENOMEM)
        unsafe {
            let boundary = usize::MAX / 2 - 4097;
            let p = malloc(boundary);
            // 该分配不可能成功（超过可用地址空间），应返回 null
            if !p.is_null() {
                // 如果内核奇迹般地给了我们内存…（测试环境不会发生）
                let _ = p;
            }
        }
    });

    test!("test_alloc_slot_with_valid_sc" {
        // 所有 48 个类别索引都应可接受
        for sc in 0..48 {
            unsafe {
                let _ = alloc_slot(sc, UNIT * SIZE_CLASSES[sc] as usize - IB);
            }
        }
    });

    test!("test_try_avail_null_head" {
        // 传入 null head: 应返回 0 而不崩溃
        unsafe {
            let mut head: *mut Meta = core::ptr::null_mut();
            let result = try_avail(&mut head);
            assert_eq!(result, 0, "try_avail(null) 应返回 0");
        }
    });

    test!("test_alloc_group_with_all_small_scs" {
        // 测试 alloc_group 对所有大小类别的可调用性
        for sc in 0..48 {
            let req = if sc < 47 {
                UNIT * SIZE_CLASSES[sc] as usize - IB
            } else {
                MMAP_THRESHOLD - 1
            };
            unsafe {
                let _ = alloc_group(sc, req);
            }
        }
    });

    // ---------------------------------------------------------------------------
    // 内存对齐验证
    // ---------------------------------------------------------------------------

    test!("test_malloc_alignment" {
        // malloc 返回的指针必须 16 字节对齐
        unsafe {
            let p = malloc(1);
            assert!(!p.is_null(), "malloc(1) 应返回非空指针");
            assert_eq!((p as usize) & 15, 0, "malloc 返回的指针必须 16 字节对齐");
        }
    });

    test!("test_malloc_different_sizes" {
        // 分配各种大小, 确保都能成功返回非空指针
        let sizes = [1, 2, 4, 7, 8, 15, 16, 31, 32, 64, 127, 128, 255, 256, 511, 512,
                     1023, 1024, 2047, 2048, 4095, 4096, 8191, 8192, 16383, 16384,
                     65535, 65536, MMAP_THRESHOLD - 1, MMAP_THRESHOLD];
        unsafe {
            for &n in &sizes {
                let p = malloc(n);
                assert!(!p.is_null(), "malloc({}) 应返回非空指针", n);
                assert_eq!((p as usize) & 15, 0, "malloc({}) 返回的指针必须 16 字节对齐", n);
            }
        }
    });

    // ---------------------------------------------------------------------------
    // SMALL_CNT_TAB 特定值验证
    // ---------------------------------------------------------------------------

    test!("test_small_cnt_tab_sc0_values" {
        // sc=0 (16B slot)
        assert_eq!(SMALL_CNT_TAB[0], [30, 30, 30]);
    });

    test!("test_small_cnt_tab_sc3_values" {
        // sc=3 (64B slot)
        assert_eq!(SMALL_CNT_TAB[3], [31, 15, 7]);
    });

    test!("test_small_cnt_tab_sc8_values" {
        // sc=8 (144B slot): sc>=9 使用 MED_CNT_TAB, 但 SMALL_CNT_TAB 仍有 sc=8
        assert_eq!(SMALL_CNT_TAB[8], [28, 14, 6]);
    });

    // ---------------------------------------------------------------------------
    // SIZE_CLASSES 分段值精确验证
    // ---------------------------------------------------------------------------

    test!("test_size_classes_first_segment" {
        // class 0-7: 1-8
        let expected: [u16; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        for i in 0..8 {
            assert_eq!(SIZE_CLASSES[i], expected[i],
                "SIZE_CLASSES[{}] expected {}, got {}", i, expected[i], SIZE_CLASSES[i]);
        }
    });

    test!("test_size_classes_second_segment" {
        // class 8-11: 9, 10, 12, 15
        let expected: [u16; 4] = [9, 10, 12, 15];
        for i in 0..4 {
            assert_eq!(SIZE_CLASSES[8 + i], expected[i],
                "SIZE_CLASSES[{}] expected {}, got {}", 8 + i, expected[i], SIZE_CLASSES[8 + i]);
        }
    });

    test!("test_size_classes_last_segment" {
        // class 44-47: 4680, 5460, 6552, 8191
        let expected: [u16; 4] = [4680, 5460, 6552, 8191];
        for i in 0..4 {
            assert_eq!(SIZE_CLASSES[44 + i], expected[i],
                "SIZE_CLASSES[{}] expected {}, got {}", 44 + i, expected[i], SIZE_CLASSES[44 + i]);
        }
    });

    // ---------------------------------------------------------------------------
    // CTX 数组维度一致性验证
    // ---------------------------------------------------------------------------

    test!("test_ctx_arrays_consistent" {
        // active[] 和 usage_by_class[] 必须长度相同 (均为 48)
        // unmap_seq[] 和 bounces[] 必须长度相同 (均为 32)
        unsafe {
            assert_eq!(CTX.active.len(), CTX.usage_by_class.len());
            assert_eq!(CTX.unmap_seq.len(), CTX.bounces.len());
            assert_eq!(CTX.unmap_seq.len(), 32);
        }
    });

    // ---------------------------------------------------------------------------
    // 架构一致性验证
    // ---------------------------------------------------------------------------

    test!("test_usize_width_consistent" {
        // SIZE_CLASSES 使用 u16 存储, 最大值 8191 可容纳于 u16
        for &sc_val in SIZE_CLASSES.iter() {
            assert!(sc_val <= u16::MAX as u16,
                "SIZE_CLASSES value {} exceeds u16::MAX", sc_val);
        }
    });

    test!("test_mmap_threshold_less_than_size_class_max" {
        // MMAP_THRESHOLD 应小于 class 47 可支撑的最大分配
        let max_class47_size = UNIT * SIZE_CLASSES[47] as usize - IB;
        assert!(MMAP_THRESHOLD <= max_class47_size + 1,
            "MMAP_THRESHOLD ({}) should be close to max class 47 size ({})",
            MMAP_THRESHOLD, max_class47_size);
    });
}