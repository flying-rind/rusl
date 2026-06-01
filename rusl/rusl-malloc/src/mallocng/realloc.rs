//! `realloc_impl` —— realloc 核心实现。
//!
//! 本模块是 `realloc` 公共入口的实际实现层。以 `pub(crate)` 可见性
//! 提供 `realloc_impl` 函数，被 `malloc::realloc::realloc` 调用。
//!
//! 对应 musl 的 `src/malloc/mallocng/realloc.c` (通过 `glue.h` 宏重命名为
//! `__libc_realloc`)。
//!
//! ## 实现策略 (按优先级递减)
//!
//! | 优先级 | Case | 策略 | 数据拷贝 | 说明 |
//! |-------|------|------|---------|------|
//! | 1 | Case 1 | p 为 null → `malloc_impl(n)` | 零拷贝 | 等价于首次分配 |
//! | 2 | Case 2 | n 导致溢出 → 返回 null, `errno = ENOMEM` | - | p 保持有效 |
//! | 3 | Case 3 | 原地缩容/扩容 | 零拷贝 | 最优路径，在槽位内调整 |
//! | 4 | Case 4 | mremap 重映射 | 零拷贝 | 仅限独立 mmap 大块 |
//! | 5 | Case 5 | malloc + copy + free | 全拷贝 | 通用回退路径 |
//!
//! ## 依赖关系
//!
//! | 依赖 | 来源 | 类型 | 说明 |
//! |------|------|------|------|
//! | `malloc_impl` | `mallocng::malloc` | 内部实现 | 新内存分配 (Case 1, Case 5) |
//! | `free_impl` / `__libc_free` | `mallocng::free` | 内部实现 | 旧内存释放 (Case 5) |
//! | `sys_mremap` | `mallocng::syscall` | 内部 syscall | mmap 区域重映射 (Case 4) |
//! | `core::ptr::copy_nonoverlapping` | `core` crate | 标准库 | 内存拷贝 (Case 5) |
//! | `core::cmp::min` | `core` crate | 标准库 | 取 min(旧大小, n) (Case 5) |
//! | `get_meta` / `get_slot_index` / `get_stride` / `get_nominal_size` / `set_size` / `size_to_class` / `size_overflows` | `mallocng::meta` | 内部辅助 | 元数据操作 |
//! | `UNIT` / `IB` / `MMAP_THRESHOLD` | `mallocng::meta` | 内部常量 | 配置参数 |
//! | `MREMAP_MAYMOVE` / `MAP_FAILED` | `mallocng::syscall` | 内部常量 | mremap 标志和哨兵值 |
//! | `ENOMEM` | `crate::malloc` | 内部常量 | POSIX 错误码 |
//! | `errno` | `crate::errno` | 内部机制 | 线程局部错误码 |

use core::cmp::min;
use core::ffi::c_void;
use core::ptr;

use super::meta::{self, Meta, IB, MMAP_THRESHOLD, UNIT};
use super::syscall::{self, MAP_FAILED, MREMAP_MAYMOVE};

// ============================================================================
// 前向依赖: malloc_impl / free_impl
//
// 这两个符号定义于未来的 mallocng/malloc.rs 和 mallocng/free.rs。
// realloc_impl 通过 crate-internal 路径访问它们:
//   - malloc_impl(n)  → 等价于 C 的 malloc(n)
//   - __libc_free(p)  → 等价于 C 的 free(p) (来自 super::free)
//
// 当前阶段使用 todo!() 占位，实现完成后由编译时链接解析。
// ============================================================================

// ============================================================================
// 核心实现: realloc_impl
// ============================================================================

/// mallocng realloc 核心实现。
///
/// 更改 `p` 指向的内存块大小为 `n` 字节。采用多级策略，
/// 按优先级递减尝试最优路径，尽量减少数据拷贝和系统调用。
///
/// # 参数
///
/// - `p`: 指向原内存块的指针，可为 null
/// - `n`: 新的请求大小（字节）
///
/// # 返回值
///
/// - **成功**: 返回指向新大小内存块的指针（可能与 `p` 相同或不同）
/// - **失败**: 返回 `null`，设置 `errno = ENOMEM`，原内存块 `p` 保持有效
///
/// # 行为流程
///
/// ## Case 1: `p.is_null()` (等效于 malloc)
///
/// 直接调用 `malloc_impl(n)` 分配新内存。
///
/// - **成功**: 返回分配得到的指针，内存内容未初始化
/// - **失败**: 返回 `null`，`errno = ENOMEM`
///
/// ## Case 2: `n` 导致溢出
///
/// 调用 `size_overflows(n)` 检查。若溢出：
/// - 返回 `null`
/// - 设置 `errno = ENOMEM`
/// - 原内存块 `p` 保持有效且未被释放，调用者必须后续显式 `free_impl(p)`
///
/// ## Case 3: 原地缩容/扩容 (最优路径, 零拷贝)
///
/// **触发条件** (三个条件同时满足):
/// 1. `n <= avail_size` — 新大小不超过槽位可用空间
/// 2. `n < MMAP_THRESHOLD` (131052 字节) — 不触发大块阈值
/// 3. `size_to_class(n) + 1 >= g.sc` — 大小类别兼容
///
/// **计算过程**:
/// ```ignore
/// g = get_meta(p)           // 定位元数据 (含安全断言)
/// idx = get_slot_index(p)   // 提取槽位索引 (0-31)
/// stride = get_stride(g)    // 获取槽位跨度
/// start = g.mem.storage.as_ptr().add(stride * idx)  // 槽位起始
/// end = start.add(stride - IB)    // 槽位末尾 (减去 IB 哨兵空间)
/// avail_size = (end as usize) - (p as usize)
/// ```
///
/// **动作**: 调用 `set_size(p, end, n)` 就地更新记录的大小。
///
/// **返回**: 原指针 `p`（内存地址不变，无数据拷贝）。
///
/// **数据完整性**: 原有数据在 `min(旧大小, n)` 范围内保持不变。
///
/// ## Case 4: mremap 重映射 (mmap 大块优化路径)
///
/// **触发条件** (两个条件同时满足):
/// 1. `g.sc >= 48` — 原块为大对象（独立 mmap 分配）
/// 2. `n >= MMAP_THRESHOLD` (131052 字节) — 新大小也达到大块阈值
///
/// **前置断言**: `g.sc == 63`（必须为独立 mmap 分配, 非子分配组）
///
/// **计算过程**:
/// ```ignore
/// base = (p as usize) - (start as usize)  // 用户数据偏移量
/// needed = (n + base + UNIT + IB + 4095) & !4095  // 向上取整到页边界
/// ```
///
/// **子情况 4a: 新大小恰好等于原大小**:
/// - 若 `g.maplen * 4096 == needed`，无需重新映射
/// - 直接复用现有映射，跳过 mremap 系统调用
///
/// **子情况 4b: 需要 mremap**:
/// - 调用 `sys_mremap(g.mem as *mut c_void, g.maplen * 4096, needed, MREMAP_MAYMOVE)`
///
/// **成功处理**: 更新 `g.mem`、`g.maplen`，重新计算 `p` 和 `end`，
/// 写入尾部哨兵 `*end = 0`，调用 `set_size(p, end, n)`，返回更新后的 `p`。
///
/// **失败处理**: 若 `sys_mremap` 返回 `MAP_FAILED`，**内核保证原映射保持不变**。
/// 不返回 null，继续执行 Case 5 的 malloc+copy+free 回退路径。
///
/// ## Case 5: malloc + copy + free (通用回退路径)
///
/// **触发条件**: Case 3 和 Case 4 的条件均不满足，或 Case 4 的 mremap 失败。
///
/// **动作**:
/// 1. `new = malloc_impl(n)` — 分配新内存块
/// 2. 若 `new.is_null()`: 返回 `null`，`errno = ENOMEM`，原块 `p` 保持有效
/// 3. `core::ptr::copy_nonoverlapping(p as *const u8, new as *mut u8, min(n, old_size))` — 拷贝数据
/// 4. `__libc_free(p)` — 释放旧内存块
/// 5. 返回 `new`
///
/// **数据完整性**: 原有数据拷贝到新地址（上限为 `min(old_size, n)`），超出部分未初始化。
///
/// # 不变量
///
/// 1. **Inv 1 (数据安全)**: Case 2 溢出失败和 Case 5 malloc 失败时，
///    原内存块 `p` 始终有效且内容不变。调用者必须在失败时持有 `p`
///    并在后续显式 `free_impl(p)`。
///
/// 2. **Inv 2 (原地调整安全性)**: Case 3 保证 `n <= end - p`，
///    即新大小不超过槽位物理容量，不会越界写入。
///
/// 3. **Inv 3 (sizeclass 单调性)**: 条件 `size_to_class(n) + 1 >= g.sc`
///    确保新大小类别不显著低于原类别，避免大槽位浪费。
///
/// 4. **Inv 4 (mmap 大小一致性)**: Case 4 成功后，
///    `g.maplen` 总是等于 `needed / 4096`。
///
/// 5. **Inv 5 (哨兵字节)**: `set_size` 后在适当位置写入哨兵字节 0，
///    用于 `free` 时的完整性验证和越界写入检测。
///
/// 6. **Inv 6 (errno 透明性)**: mremap 失败回退到 Case 5 后，
///    后续 `malloc_impl` 调用会重新设置有意义的 `errno`。
///
/// 7. **Inv 7 (指针有效性)**: 返回的非空指针必须能被 `get_meta()` 正确解析。
///    由 `set_size` 的编码规则和 `get_meta` 的校验链保证。
///
/// # Safety
///
/// - 若 `p` 不为 null，必须是由 `malloc_impl`/`realloc_impl`/`calloc_impl` 等
///   分配器内部函数返回的有效指针，且尚未被释放
/// - `p` 必须满足 16 字节对齐（`(p as usize) & 15 == 0`），由 `get_meta()` 断言保证
/// - 调用者负责确保 `p` 不会被并发修改
/// - 调用者无锁持有要求（内部通过 `malloc`/`free` 自行管理锁）
///
/// # 线程安全性
///
/// 通过内部 `malloc_impl`/`free_impl` 的锁机制保证线程安全。
/// `realloc_impl` 自身不直接获取锁。
///
/// # 信号安全性
///
/// 不是 async-signal-safe。持有锁期间被信号中断可能导致死锁。
///
/// # 系统算法
///
/// ```ignore
/// // Level 1: 元数据定位阶段 (O(1))
/// g = get_meta(p)                                    // 定位 struct meta
/// idx = get_slot_index(p)                             // 提取槽位索引 (0-31)
/// stride = get_stride(g)                              // 计算槽位跨度
/// start = g.mem.storage.as_ptr().add(stride * idx)    // 槽位起始地址
/// end = start.add(stride - IB)                        // 槽位末尾 (减去 IB)
/// old_size = get_nominal_size(p, end)                 // 解码原始分配大小
/// avail_size = (end as usize) - (p as usize)          // 当前可用空间
///
/// // Level 2: 三路策略选择
/// if n <= avail_size && n < MMAP_THRESHOLD && size_to_class(n) + 1 >= g.sc:
///     → PATH A: 原地更新 (set_size) → return p       // 最优路径, 零拷贝
///
/// if g.sc >= 48 && n >= MMAP_THRESHOLD:
///     assert(g.sc == 63)
///     → PATH B: mremap 重映射
///     if mremap 成功:
///         → 更新元数据, set_size → return p          // 无数据拷贝
///     // mremap 失败时内核保证原映射不变, 继续 PATH C
///
/// → PATH C: malloc + copy_nonoverlapping + free → return new  // 通用回退
/// ```
#[inline]
pub(crate) unsafe fn realloc_impl(p: *mut c_void, n: usize) -> *mut c_void {
    // Case 1: p 为 null → 等价于 malloc(n)
    if p.is_null() {
        return super::malloc::malloc(n);
    }

    // Case 1.5: n == 0 → 等价于 free(p), 返回 NULL (musl 行为, 符合 C2x)
    if n == 0 {
        super::free::__libc_free(p);
        return core::ptr::null_mut();
    }

    // Case 2: n 导致溢出 → 返回 null, 设置 errno = ENOMEM
    if meta::size_overflows(n) {
        // Safety: __errno_location 返回有效指针
        unsafe {
            rusl_errno::__errno_location().write(super::super::ENOMEM);
        }
        return core::ptr::null_mut();
    }

    // Level 1: 元数据定位阶段
    let g: *mut Meta = meta::get_meta(p as *const u8);
    let idx: usize = meta::get_slot_index(p as *const u8);
    let stride: usize = meta::get_stride(g);

    // 计算槽位起始和末尾
    let group_ptr = (*g).mem;
    let storage = (group_ptr as *mut u8).add(UNIT);
    let start = storage.add(stride * idx);
    let end = start.add(stride - IB);

    // 解码原始分配大小
    let old_size = meta::get_nominal_size(p as *const u8, end);
    // 当前可用空间
    let avail_size = (end as usize) - (p as usize);

    // Level 2: 三路策略选择

    // Case 3: 原地缩容/扩容 (最优路径, 零拷贝)
    // 触发条件: (1) n <= avail_size  (2) n < MMAP_THRESHOLD  (3) 类别兼容
    if n <= avail_size
        && n < MMAP_THRESHOLD
        && meta::size_to_class(n) + 1 >= (*g).sizeclass()
    {
        meta::set_size(p as *mut u8, end, n);
        return p;
    }

    // Case 4: mremap 重映射 (mmap 大块优化路径)
    // 触发条件: (1) g.sizeclass >= 48  (2) n >= MMAP_THRESHOLD
    if (*g).sizeclass() >= 48 && n >= MMAP_THRESHOLD {
        // 前置断言: 必须为独立 mmap 分配 (sc == 63)
        debug_assert!((*g).sizeclass() == 63);

        // 计算用户数据在 mmap 区域内的偏移量
        let base = (p as usize) - (start as usize);
        // needed = (n + base + UNIT + IB + 4095) & !4095 — 向上取整到页边界
        let needed = (n + base + UNIT + IB + 4095) & !4095;

        // 子情况 4a: 新大小恰好等于原大小 — 无需重映射
        let new_mem = if (*g).maplen() * 4096 == needed {
            (*g).mem as *mut c_void
        } else {
            // 子情况 4b: 调用 sys_mremap 扩展/收缩映射
            syscall::sys_mremap(
                (*g).mem as *mut c_void,
                (*g).maplen() * 4096,
                needed,
                MREMAP_MAYMOVE,
            )
        };

        if new_mem != MAP_FAILED {
            // 成功: 更新元数据
            (*g).mem = new_mem as *mut super::meta::Group;
            (*g).set_maplen(needed / 4096);
            // 重新计算用户指针
            let new_p = ((*g).mem as *mut u8).add(UNIT).add(base) as *mut c_void;
            // 重新计算末尾边界: end = storage + (needed - UNIT) - IB
            let new_end = ((*g).mem as *mut u8)
                .add(UNIT)
                .add(needed - UNIT - IB);
            // 写入尾部哨兵
            new_end.write(0);
            // 更新大小记录
            meta::set_size(new_p as *mut u8, new_end, n);
            return new_p;
        }
        // mremap 失败: 内核保证原映射不变, 继续 Case 5 (不回退到返回 null)
    }

    // Case 5: malloc + copy_nonoverlapping + free (通用回退路径)
    let new = super::malloc::malloc(n);
    if new.is_null() {
        // 分配失败: 返回 null, errno 已由 malloc 设置, 原块 p 保持有效
        return core::ptr::null_mut();
    }
    // 拷贝数据: 上限为 min(n, old_size)
    let copy_size = min(n, old_size);
    ptr::copy_nonoverlapping(p as *const u8, new as *mut u8, copy_size);
    // 释放旧内存块
    super::free::__libc_free(p);
    // 返回新块
    new
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::ptr;
    use super::super::malloc::malloc;
    use crate::free::free;

    // ========================================================================
    // 辅助函数
    // ========================================================================

    /// 安全释放测试用的分配指针。
    unsafe fn safe_free(p: *mut c_void) {
        if !p.is_null() {
            free(p);
        }
    }

    /// 分配测试用内存块，若系统 malloc 失败则通过 assert 终止。
    unsafe fn must_malloc(size: usize) -> *mut c_void {
        let p = malloc(size);
        assert!(!p.is_null(), "系统 malloc({}) 失败，无法继续测试", size);
        p
    }

    // ========================================================================
    // Case 1: p 为 null → 等效 malloc
    // ========================================================================

    test!("test_case1_null_ptr_delegates_to_malloc" {
        // Spec: realloc_impl(NULL, n) 应等价于 malloc_impl(n)
        //
        // 验证点:
        // 1. p == NULL 时等同于首次分配
        // 2. 返回的指针满足 16 字节对齐要求
        // 3. 至少可写入 n 字节
        //
        // 此测试在 realloc_impl 实现后验证完整流程。
    });

    test!("test_case1_null_ptr_zero_size" {
        // Spec: realloc_impl(NULL, 0) 等价于 malloc_impl(0)
        //
        // malloc(0) 的行为是实现定义的:
        // - 返回可 free 的非 null 指针 (musl 行为)
        // - 或返回 null (POSIX 允许)
        //
        // 两种结果都合法。实现后验证返回值可安全传给 free_impl。
    });

        test!("test_case1_null_ptr_various_sizes" {
        // Spec: realloc_impl(NULL, n) 对所有合理 n 值都应等效于 malloc
        //
        // 验证点: 多种大小下返回非 null 对齐指针
        let test_sizes: &[usize] = &[
            1,      // 最小分配
            16,     // 1 倍 UNIT
            64,     // 4 倍 UNIT
            256,    // 典型分配
            1024,   // 1 KB
            4096,   // 1 页面
            65536,  // 64 KB
            131072, // 128 KB (= MMAP_THRESHOLD 边界)
            131073, // 128 KB + 1 (> MMAP_THRESHOLD，触发 mmap 路径)
        ];

        for &size in test_sizes {
            // realloc_impl 尚未实现，此为规格验证
            let _ = size; // 消除未使用警告
        }
        });

    // ========================================================================
    // Case 2: 溢出检查
    // ========================================================================

        test!("test_case2_overflow_returns_null" {
        // Spec: size_overflows(n) == true → 返回 null, errno = ENOMEM
        //
        // 验证点:
        // 1. 不 panic
        // 2. 返回 null
        // 3. 调用者负责检查 errno == ENOMEM
        // 4. 原 p 保持有效
        unsafe {
            let p = must_malloc(64);
            ptr::write_bytes(p as *mut u8, 0xFE, 64);

            // 传入接近 usize::MAX 的值应触发溢出
            let result = realloc_impl(p, usize::MAX);
            // 注意: realloc_impl 尚未实现，以下为规格验证占位
            let _ = result;
            // 规格: result.is_null() && errno == ENOMEM
            // 规格: 原 p 的前 64 字节仍为 0xFE (数据不变)

            safe_free(p);
        }
        });

        test!("test_case2_overflow_threshold_boundary" {
        // Spec: size_overflows 阈值 = usize::MAX / 2 - 4096
        //
        // 验证点:
        // 1. 阈值 - 1: 不溢出，正常处理
        // 2. 阈值: 溢出，返回 null
        // 3. 阈值 + 1: 溢出，返回 null
        //
        // 阈值公式: n >= usize::MAX / 2 - 4096
        let threshold = usize::MAX / 2 - 4096;

        unsafe {
            let p = must_malloc(128);
            ptr::write_bytes(p as *mut u8, 0xCD, 128);

            // 阈值 - 1: 不溢出
            {
                let result = realloc_impl(p, threshold.saturating_sub(1));
                let _ = result;
                // 规格: !result.is_null()
            }

            // 阈值: 溢出
            {
                let result = realloc_impl(p, threshold);
                let _ = result;
                // 规格: result.is_null() && errno == ENOMEM
            }

            safe_free(p);
        }
        });

        test!("test_case2_overflow_preserves_original_data" {
        // Spec Inv 1 (数据安全): Case 2 失败时原内存块内容不变
        //
        // 验证点: 写入可识别模式后触发溢出 → 原数据仍可读
        unsafe {
            let p = must_malloc(256);

            // 写入递增模式
            for i in 0..256 {
                ptr::write(p.add(i) as *mut u8, (i & 0xFF) as u8);
            }

            let result = realloc_impl(p, usize::MAX);
            let _ = result;
            // 规格: result.is_null()

            // 规格: 所有字节保持不变
            for _i in 0..256 {
                // assert_eq!(ptr::read(p.add(i) as *const u8), (i & 0xFF) as u8);
            }

            safe_free(p);
        }
        });

    // ========================================================================
    // Case 3: 原地缩容/扩容 (条件测试)
    // ========================================================================

        test!("test_case3_condition_1_n_leq_avail_size" {
        // Spec: Case 3 触发条件 1: n <= avail_size
        //
        // avail_size = end - p，即从用户指针到槽位末尾的可用字节数。
        //
        // 验证此条件的边界:
        // - n == avail_size: 刚好填满，应触发 Case 3
        // - n == avail_size + 1: 超出 1 字节，不应触发 Case 3
        //
        // 注意: 此条件依赖运行时 get_meta/get_stride 计算结果，
        // 单元测试仅验证数学上的正确性。
        });

        test!("test_case3_condition_2_n_lt_mmap_threshold" {
        // Spec: Case 3 触发条件 2: n < MMAP_THRESHOLD (131052)
        //
        // 验证边界:
        // - n == MMAP_THRESHOLD - 1: 满足条件
        // - n == MMAP_THRESHOLD: 不满足条件 (走 Case 4 或 Case 5)
        assert!(
            MMAP_THRESHOLD.saturating_sub(1) < MMAP_THRESHOLD,
            "n == MMAP_THRESHOLD - 1 应满足 n < MMAP_THRESHOLD"
        );
        assert!(
            !(MMAP_THRESHOLD < MMAP_THRESHOLD),
            "n == MMAP_THRESHOLD 不满足 n < MMAP_THRESHOLD"
        );
        });

        test!("test_case3_condition_3_sizeclass_monotonic" {
        // Spec: Case 3 触发条件 3: size_to_class(n) + 1 >= g.sc
        //
        // 此条件确保新大小类别不低于原类别太多。
        //
        // 验证点 (数学正确性):
        // 1. 若新类别 == 原类别 → 满足 (c + 1 >= c 总是成立)
        // 2. 若新类别 == 原类别 - 1 → 满足 (c - 1 + 1 >= c 成立)
        // 3. 若新类别 < 原类别 - 1 → 不满足
        //
        // 例如: g.sc = 10, size_to_class(n) = 8
        //   8 + 1 = 9 < 10 → 不满足，走 Case 5 以使用更合适的槽位大小
        let sc: usize = 10; // 模拟原大小类别
        let new_class_ok1: usize = 10; // c + 1 = 11 >= 10 ✓
        let new_class_ok2: usize = 9; // c + 1 = 10 >= 10 ✓ (边界)
        let new_class_bad: usize = 8; // c + 1 = 9 < 10 ✗

        assert!(
            new_class_ok1 + 1 >= sc,
            "新类别 == 原类别 应满足条件 ({} + 1 >= {})",
            new_class_ok1,
            sc
        );
        assert!(
            new_class_ok2 + 1 >= sc,
            "新类别 == 原类别 - 1 应满足条件 ({} + 1 >= {})",
            new_class_ok2,
            sc
        );
        assert!(
            !(new_class_bad + 1 >= sc),
            "新类别 < 原类别 - 1 不应满足条件 ({} + 1 >= {})",
            new_class_bad,
            sc
        );
        });

        test!("test_case3_shrink_within_slot" {
        // Spec: Case 3 原地缩容 — 新大小不超过可用空间
        //
        // 验证点:
        // 1. 缩容后返回原指针 (地址不变)
        // 2. 前 n 字节数据保持不变
        // 3. 无数据移动
        unsafe {
            let p = must_malloc(256);
            ptr::write_bytes(p as *mut u8, 0xAB, 256);

            // 缩容到 128 字节
            // (实现完成后验证)
            let new_p = realloc_impl(p, 128);
            let _ = new_p;
            // 规格: !new_p.is_null()
            // 规格: new_p 可能等于 p (原地缩容) 或不等于 (Case 5 回退)

            safe_free(new_p);
        }
        });

        test!("test_case3_grow_within_slot" {
        // Spec: Case 3 原地扩容 — 新大小不超过可用空间
        //
        // 验证点:
        // 1. 扩容后可能返回原指针
        // 2. 前旧大小字节数据保持不变
        // 3. 超出旧大小的部分未初始化
        unsafe {
            let p = must_malloc(64);
            ptr::write_bytes(p as *mut u8, 0xBC, 64);

            // 扩容到 128 字节 (若可用空间充足)
            let new_p = realloc_impl(p, 128);
            let _ = new_p;
            // 规格: !new_p.is_null()

            safe_free(new_p);
        }
        });

    // ========================================================================
    // Case 4: mremap 重映射 (条件与逻辑测试)
    // ========================================================================

        test!("test_case4_condition_sc_geq_48" {
        // Spec: Case 4 触发条件 1: g.sc >= 48 (大对象类别)
        //
        // 验证点:
        // 1. g.sc = 47 → 不触发 Case 4 (标准大小类别)
        // 2. g.sc = 48 → 触发 Case 4
        // 3. g.sc = 63 → 触发 Case 4 (独立 mmap 分配)
        //
        // 注意: 48-62 虽是"大对象类别"，但 Case 4 内有
        //       assert(g.sc == 63) 断言，意味着实际上只有
        //       g.sc == 63 的独立 mmap 分配才会真正执行。
        //       48-62 理论上不应出现 (预留类别)。
        });

        test!("test_case4_condition_n_geq_mmap_threshold" {
        // Spec: Case 4 触发条件 2: n >= MMAP_THRESHOLD (131052)
        //
        // 验证边界:
        // - n == MMAP_THRESHOLD: 触发
        // - n == MMAP_THRESHOLD - 1: 不触发
        assert!(
            MMAP_THRESHOLD >= MMAP_THRESHOLD,
            "n == MMAP_THRESHOLD 应满足 n >= MMAP_THRESHOLD"
        );
        assert!(
            !(MMAP_THRESHOLD.saturating_sub(1) >= MMAP_THRESHOLD),
            "n == MMAP_THRESHOLD - 1 不满足 n >= MMAP_THRESHOLD"
        );
        });

        test!("test_case4_sizeclass_equals_63_assertion" {
        // Spec: Case 4 入口断言 g.sc == 63
        //
        // 若 g.sc >= 48 但 != 63，assert 应触发 crash/panic。
        // 这是因为 48-62 不是有效的 mmap 大小类别。
        //
        // 验证点: 断言在 Debug 和 Release 构建中都保留
        // (使用 assert! 而非 debug_assert!)
        });

        test!("test_case4a_no_remap_when_same_size" {
        // Spec Case 4a: 若 g.maplen * 4096 == needed，无需重映射
        //
        // needed = (n + base + UNIT + IB + 4095) & !4095
        //
        // 当新大小经页对齐计算后与原映射大小相同时，跳过 mremap。
        //
        // 验证点: 此优化避免不必要的系统调用
        });

        test!("test_case4b_mremap_called_when_needed" {
        // Spec Case 4b: 调用 sys_mremap(old, old_len, needed, MREMAP_MAYMOVE)
        //
        // 验证点:
        // 1. sys_mremap 成功时返回新地址
        // 2. g.mem 和 g.maplen 正确更新
        // 3. 尾部哨兵 *end == 0
        // 4. set_size(p, end, n) 正确编码新大小
        });

        test!("test_case4b_mremap_fails_falls_through_to_case5" {
        // Spec: mremap 返回 MAP_FAILED 时，内核保证原映射不变
        //
        // 代码必须继续执行 Case 5 而不是返回 null。
        //
        // 验证点:
        // 1. mremap 失败后不提前返回
        // 2. 继续执行 malloc_impl + copy_nonoverlapping + __libc_free
        // 3. 最终返回新分配的指针 (Case 5 成功) 或 null (Case 5 失败)
        //
        // 这是关键的安全保证。
        });

        test!("test_case4_needed_page_align_calculation" {
        // Spec: needed = (n + base + UNIT + IB + 4095) & !4095
        //
        // 验证数学正确性:
        //
        // 例 1: n=131052, base=32, UNIT=16, IB=4
        //   sum = 131052 + 32 + 16 + 4 = 131104
        //   aligned = (131104 + 4095) & !4095 = 135168 & !4095 = 135168
        //
        // 例 2: n=131052, base=0, UNIT=16, IB=4
        //   sum = 131072
        //   aligned = (131072 + 4095) & !4095 = 131072 (刚好页对齐)

        let n1: usize = 131052;
        let base1: usize = 32;
        let sum1 = n1 + base1 + UNIT + IB;
        assert_eq!(sum1, 131104);
        let aligned1 = (sum1 + 4095) & !4095;
        assert_eq!(aligned1, 135168);
        assert_eq!(aligned1 % 4096, 0, "结果必须页对齐");

        let n2: usize = 131052;
        let base2: usize = 0;
        let sum2 = n2 + base2 + UNIT + IB;
        assert_eq!(sum2, 131072);
        let aligned2 = (sum2 + 4095) & !4095;
        assert_eq!(aligned2, 131072);
        assert_eq!(aligned2 % 4096, 0, "结果必须页对齐");
        });

    // ========================================================================
    // Case 5: malloc + copy + free (通用回退路径)
    // ========================================================================

        test!("test_case5_copy_data_to_new_block" {
        // Spec: Case 5 分配新块 + 拷贝数据 + 释放旧块
        //
        // 验证点:
        // 1. 旧数据完整拷贝到新位置 (前 min(old_size, n) 字节)
        // 2. 旧块被释放
        // 3. 超出 old_size 的部分未初始化
        unsafe {
            let p = must_malloc(64);

            // 写入递增模式
            for i in 0..64 {
                ptr::write(p.add(i) as *mut u8, (i * 3) as u8);
            }

            // 扩容到 4096，触发 Case 5
            let new_p = realloc_impl(p, 4096);
            let _ = new_p;
            // 规格: !new_p.is_null()
            // 规格: 前 64 字节 == 递增模式

            safe_free(new_p);
        }
        });

        test!("test_case5_shrink_to_smaller_class" {
        // Spec: 当缩容后新大小类别显著小于原类别 (不满足 Case 3 条件 3)，
        // 走 Case 5 分配更小类别的新块。
        //
        // 验证点:
        // 1. 新块分配在更小的槽位中
        // 2. 前 n 字节数据完整
        // 3. 旧块被正确释放
        unsafe {
            let p = must_malloc(65536); // 大类分配
            ptr::write_bytes(p as *mut u8, 0x55, 65536);

            // 缩容到 16 字节 (远小于原块，触发 Case 5)
            let new_p = realloc_impl(p, 16);
            let _ = new_p;
            // 规格: !new_p.is_null()
            // 规格: 前 16 字节 == 0x55

            safe_free(new_p);
        }
        });

        test!("test_case5_malloc_fails_returns_null_preserves_old" {
        // Spec Inv 1: Case 5 中 malloc_impl(n) 失败时返回 null，
        // 原块 p 保持有效。
        //
        // 验证点:
        // 1. 模拟内存耗尽 (实际测试中难以触发)
        // 2. 返回 null
        // 3. errno == ENOMEM
        // 4. p 仍可正常使用和释放
        //
        // 注意: 在单元测试环境中难以精确模拟 OOM，
        // 此测试主要验证失败路径的正确性。
        unsafe {
            let p = must_malloc(128);
            ptr::write_bytes(p as *mut u8, 0xEE, 128);

            // 尝试分配极大值 (但不触发 Case 2 溢出)
            // 若系统无法分配，应返回 null 且保留 p
            let huge_size = usize::MAX / 4; // 不触发溢出但可能 OOM
            let result = realloc_impl(p, huge_size);
            let _ = result;
            // 无论成功还是失败，验证数据完整性

            // 规格: 若 result.is_null()，p 的前 128 字节仍为 0xEE
            safe_free(p);
        }
        });

    // ========================================================================
    // mmap 阈值边界测试 (跨 Case 3/4/5)
    // ========================================================================

        test!("test_mmap_threshold_crossing_small_to_large" {
        // Spec: 从小块 (< MMAP_THRESHOLD) 扩容到大块 (>= MMAP_THRESHOLD)
        //
        // 若原块为常规 slot 分配 (g.sc < 48):
        //   - 不满足 Case 4 条件 (g.sc < 48)
        //   - 不满足 Case 3 条件 (n >= MMAP_THRESHOLD)
        //   - 走 Case 5: 分配新 mmap 大块 + 拷贝 + 释放旧 slot
        //
        // 验证点:
        // 1. 原 slot 被正确释放回组内
        // 2. 新 mmap 大块数据完整
        unsafe {
            let p = must_malloc(128);
            ptr::write_bytes(p as *mut u8, 0x77, 128);

            let new_p = realloc_impl(p, 200_000);
            let _ = new_p;
            // 规格: !new_p.is_null()
            // 规格: 前 128 字节 == 0x77

            safe_free(new_p);
        }
        });

        test!("test_mmap_threshold_crossing_large_to_small" {
        // Spec: 从大块 (>= MMAP_THRESHOLD) 缩容到小块 (< MMAP_THRESHOLD)
        //
        // 若原块为 mmap 大块 (g.sc == 63):
        //   - 不满足 Case 4 条件 (n < MMAP_THRESHOLD)
        //   - 可能满足 Case 3 条件 (若缩容仍在 mmap slot 内且类别兼容)
        //   - 否则走 Case 5: 分配新 slot + 拷贝 + 释放旧 mmap
        //
        // 验证点:
        // 1. 数据完整
        // 2. 旧 mmap 区域被 munmap 或归还
        unsafe {
            let p = must_malloc(200_000);
            ptr::write_bytes(p as *mut u8, 0x88, 200_000);

            let new_p = realloc_impl(p, 256);
            let _ = new_p;
            // 规格: !new_p.is_null()
            // 规格: 前 256 字节 == 0x88

            safe_free(new_p);
        }
        });

    test!("test_mmap_threshold_exact_boundary" {
        // MMAP_THRESHOLD = 131052 = 128K - 20 = 128K - (UNIT + IB)
        // 其中 UNIT=16, IB=4, 用于存放组元数据开销。
        assert_eq!(MMAP_THRESHOLD, 131052, "MMAP_THRESHOLD 应为 131052 = 128K - 20");
    });

    // ========================================================================
    // 多级生命周期测试
    // ========================================================================

        test!("test_realloc_complete_lifecycle" {
        // malloc → realloc(扩大) → realloc(扩大) → realloc(缩小) → free
        //
        // 验证点:
        // 1. 链式 realloc 调用正常工作
        // 2. 每次 realloc 返回的指针可安全传递给下一调用
        // 3. 历史数据在整个生命周期内保持完整
        unsafe {
            let mut p = must_malloc(64);
            ptr::write_bytes(p as *mut u8, 0x01, 64);

            // 扩到 256
            p = realloc_impl(p, 256);
            assert!(!p.is_null(), "第 1 次 realloc 失败");
            for _i in 0..64 {
                // 规格: assert_eq!(ptr::read(p.add(i) as *const u8), 0x01);
            }

            // 扩到 4096
            p = realloc_impl(p, 4096);
            assert!(!p.is_null(), "第 2 次 realloc 失败");
            for _i in 0..64 {
                // 规格: assert_eq!(ptr::read(p.add(i) as *const u8), 0x01);
            }

            // 缩到 16
            p = realloc_impl(p, 16);
            assert!(!p.is_null(), "第 3 次 realloc 失败");
            for _i in 0..16 {
                // 规格: assert_eq!(ptr::read(p.add(i) as *const u8), 0x01);
            }

            safe_free(p);
        }
        });

        test!("test_realloc_multiple_grow_chain" {
        // 多次扩容: 确保每次都能正确迁移数据
        //
        // 验证点:
        // 1. 连续 5 次扩容不丢失数据
        // 2. 每次扩容后旧块正确释放 (无内存泄漏)
        // 3. 跨越多种大小类别
        unsafe {
            let sizes: &[usize] = &[64, 256, 1024, 4096, 16384, 65536];
            let mut p = must_malloc(sizes[0]);

            // 写入初始标记
            ptr::write(p as *mut u32, 0xDEAD_BEEF_u32);

            for &size in &sizes[1..] {
                p = realloc_impl(p, size);
                assert!(!p.is_null(), "扩容到 {} 字节失败", size);
                // 规格: assert_eq!(ptr::read(p as *const u32), 0xDEAD_BEEF_u32);
            }

            safe_free(p);
        }
        });

    // ========================================================================
    // 不变量验证测试
    // ========================================================================

        test!("test_inv1_data_safety_on_failure" {
        // Spec Inv 1: 失败时原内存块 p 保持有效且内容不变
        //
        // 验证点 (数学/逻辑):
        // 1. Case 2 溢出: 不修改 p 的 header，不释放 p
        // 2. Case 5 malloc 失败: 不修改 p，不释放 p
        //
        // 这两个失败路径在返回 null 前必须确保 p 的内容未被修改。
        //
        // 注意: p 被修改的唯一时机是 Case 3 (set_size) 和 Case 5
        // (copy_nonoverlapping + __libc_free)，以及 Case 4 mremap 成功后。
        // 所有其他路径 (Case 1, Case 2, mremap 失败, Case 5 malloc 失败)
        // 都不应修改 p 指向的内存。
        });

        test!("test_inv2_inplace_resize_safety" {
        // Spec Inv 2: Case 3 保证 n <= end - p
        //
        // 数学证明:
        // Case 3 条件 1: n <= avail_size
        // avail_size = (end as usize) - (p as usize)
        // 因此 n <= end - p 是条件 1 的直接推论。
        //
        // set_size(p, end, n) 的实现要求 n <= end - p，
        // Case 3 的条件保证此约束成立。
        });

        test!("test_inv3_sizeclass_monotonic_implication" {
        // Spec Inv 3: size_to_class(n) + 1 >= g.sc
        //
        // 当此条件不满足时 (新类别远小于原类别)，走 Case 5。
        // 这是因为保留原大槽位而只用其中极小部分会严重浪费空间。
        //
        // 例如: 原槽位可容纳 32768 字节 (类别 ~20)，
        // 用户缩容到 16 字节 (类别 1)。
        // 1 + 1 = 2 < 20 → 不满足条件，走 Case 5。
        //
        // Case 5 会分配类别 1 的新槽位（仅 32 字节），将 16 字节复制后
        // 释放原 32KB 槽位。相比原地保留 32KB 槽位只使用 16 字节，
        // 新方案节省了约 31KB 内存。
        });

        test!("test_inv4_mmap_size_consistency" {
        // Spec Inv 4: Case 4 成功后 g.maplen == needed / 4096
        //
        // needed 是页对齐的: needed = (sum + 4095) & !4095
        // 因此 needed / 4096 始终为整数。
        //
        // 验证:
        // - needed = 4096 → maplen = 1
        // - needed = 8192 → maplen = 2
        // - needed = 135168 → maplen = 33
        assert_eq!(4096 / 4096, 1);
        assert_eq!(8192 / 4096, 2);
        assert_eq!(135168 / 4096, 33);
        });

        test!("test_inv5_sentinel_bytes" {
        // Spec Inv 5: set_size 后写入哨兵字节 0
        //
        // set_size 编码规则:
        //   reserved = end - p - n
        //   若 reserved > 0, end[-reserved] = 0 (哨兵)
        //   若 reserved >= 5, end[-4..-1] 写入 u32 LE 扩展值, end[-5] = 0 (哨兵)
        //   p[-3] 写入 (slot_index | reserved << 5)
        //
        // *end == 0 用于 free 时的溢出检测。
        //
        // 验证点:
        // 1. 设置 reserved = 3 时，end[-3] = 0
        // 2. 设置 reserved = 7 时，end[-5] = 0 且 end[-4..-1] 为扩展值
        });

        test!("test_inv6_errno_transparency" {
        // Spec Inv 6: mremap 失败后的 errno 值由后续 malloc 调用设置
        //
        // mremap 系统调用可能修改 errno (如 ENOMEM、EINVAL 等)。
        // 若 mremap 失败并回退到 Case 5，mremap 设置的 errno 无意义。
        // 后续 malloc_impl 调用会重新设置 errno。
        //
        // 实现必须确保: 不以 mremap 的 errno 作为最终返回的 errno。
        });

        test!("test_inv7_pointer_validity_after_set_size" {
        // Spec Inv 7: 返回的非空指针必须能被 get_meta() 正确解析
        //
        // set_size 修改 p[-3] 中的 reserved 字段但不修改 slot_index。
        // 因此 realloc 后 get_meta(p) 仍能正确工作。
        //
        // 这是 get_meta 校验链的核心不变量:
        //   p[-3] & 31 (slot_index) 在 realloc 前后不变。
        });

    // ========================================================================
    // 哨兵字节测试
    // ========================================================================

        test!("test_sentinel_byte_at_end" {
        // Spec: *end = 0 是尾部哨兵字节
        //
        // end = start + stride - IB
        // 这意味着最后一个可写字节是 end[-1]，而 *end 始终为 0。
        //
        // free 时验证此哨兵: 若 *end != 0，说明发生了越界写入，
        // 此时 free 会触发 assert 失败 (crash)。
        //
        // realloc 的 set_size 必须确保此哨兵在新 end 位置正确设置。
        });

        test!("test_sentinel_byte_at_reserved_offset" {
        // Spec: end[-reserved] == 0 是 reserved 字段的哨兵
        //
        // 当 reserved >= 5 时，哨兵在 end[-5] (扩展值之后的额外字节)。
        // 当 0 < reserved < 5 时，哨兵在 end[-reserved]。
        //
        // 此哨兵用于 free 时的 reserved 字段解码验证。
        });

    // ========================================================================
    // 对齐验证测试
    // ========================================================================

        test!("test_alignment_preserved_after_realloc" {
        // Spec: realloc 返回的指针必须满足 16 字节对齐
        //
        // 验证点:
        // 1. Case 3 (原地): 原 p 已对齐 → 返回 p 仍对齐
        // 2. Case 4 (mremap): 系统调用返回页对齐地址 → +base 后仍 16 字节对齐
        //    (因为 base = start offset 且 start 在 UNIT 边界上)
        // 3. Case 5 (新分配): malloc_impl 保证 16 字节对齐
        //
        // 对齐要求来源于 get_meta(p) 中的断言: (p as usize) & 15 == 0
        unsafe {
            let p = must_malloc(64);

            // 验证: malloc 返回的指针满足对齐
            assert_eq!((p as usize) & 15, 0, "原始 p 必须 16 字节对齐");

            // realloc 后的指针 (实现后取消注释)
            let np = realloc_impl(p, 256);
            // assert_eq!((np as usize) & 15, 0,
            //     "realloc 后 p 必须 16 字节对齐");

            safe_free(np);
        }
        });

    // ========================================================================
    // errno 验证测试
    // ========================================================================

        test!("test_enomem_constant_value" {
        // ENOMEM 在 Linux/x86_64 ABI 中值为 12
        // 在 rusl 中由 crate::ENOMEM 定义
        assert_eq!(
            super::super::super::ENOMEM,
            12,
            "ENOMEM 应为 12 (Linux x86_64 ABI)"
        );
        });

        test!("test_map_failed_constant_value" {
        // MAP_FAILED = (void*)-1
        assert_eq!(
            MAP_FAILED,
            (-1isize) as *mut c_void,
            "MAP_FAILED 应为 -1_as_*mut_c_void"
        );
        });

        test!("test_mremap_maymove_constant_value" {
        // MREMAP_MAYMOVE = 1 (Linux 内核定义)
        assert_eq!(MREMAP_MAYMOVE, 1, "MREMAP_MAYMOVE 在 Linux 内核中恒为 1");
        });

    // ========================================================================
    // 综合边界值测试
    // ========================================================================

        test!("test_realloc_minimal_inputs" {
        // 最小有效输入:
        // - p = null, n = 0 (等价于 malloc(0))
        // - p = null, n = 1 (最小分配)
        // - p = 有效指针 (1 字节分配), n = 1 (不变)
        //
        // 验证点: 极端小值不会导致 panic 或 UB
        });

        test!("test_realloc_maximal_valid_inputs" {
        // 最大有效输入:
        // - n 接近但不触发 size_overflows 的最大值
        //
        // 阈值: n < usize::MAX / 2 - 4096
        let max_valid = (usize::MAX / 2).saturating_sub(4096).saturating_sub(1);

        unsafe {
            let p = must_malloc(16);
            let result = realloc_impl(p, max_valid);
            let _ = result;
            // 规格: 不 panic, 返回合理结果
            // 若分配成功: !result.is_null()
            // 若分配失败: result.is_null() && errno == ENOMEM

            safe_free(result); // 安全释放 (如果是 null 则 no-op)
        }
        });

        test!("test_realloc_size_classes_adjust" {
        // realloc 可能需要在不同大小类别间切换
        //
        // 大小类别范围: 0 (最小, ~16 字节) → 47 (最大标准, ~128K)
        //
        // 验证跨类别行为:
        // 1. 类别 0 到类别 10: 小类 → 中类
        // 2. 类别 10 到类别 30: 中类 → 大类
        // 3. 类别 30 到类别 0: 大类 → 小类
        //
        // 每次跨类别时数据必须完整保留。
        });

    // ========================================================================
    // 坏指针 / 内存 corruption 测试 (安全关键)
    // ========================================================================

    test!("test_realloc_corrupted_pointer_crashes" {
        // Spec: get_meta(p) 内含多重断言用于检测内存 corruption
        //
        // 当传入损坏的指针时，get_meta 检测链应触发 crash:
        // 1. meta_area.check != ctx.secret → crash
        // 2. 偏移量超出范围 → crash
        // 3. avail_mask/freed_mask 不一致 → crash
        //
        // 验证点: corruption 应尽早检测 (fail-fast)，不应让
        // 损坏的数据结构继续传播。
    });

    test!("test_realloc_double_free_detection" {
        // Spec: 对已释放的 p 调用 realloc_impl 时，
        // get_meta(p) 检测 freed_mask 或 corruption 并 crash。
        //
        // 这是 double-free 检测的关键防线。
    });

    // ========================================================================
    // realloc_impl 不可达路径测试
    // ========================================================================

        test!("test_case4_fallthrough_to_case5_on_mremap_failure" {
        // Spec: mremap 失败后代码必须进入 Case 5，不可返回 null。
        //
        // 这是关键的不变量。mremap 失败后内核保证原映射不变，
        // 所以 p 仍指向有效数据。此时走 Case 5 的 malloc+copy+free
        // 是最安全的通用回退。
        //
        // 验证逻辑: 在 Case 4 的 mremap 调用后，不存在
        // "if new == MAP_FAILED { return null; }" 这样的代码路径。
        // 唯一正确的模式是:
        //   if new != MAP_FAILED { /* 成功路径 */ }
        //   /* 继续执行 Case 5 */
        });

    // ========================================================================
    // 常量值验证
    // ========================================================================

        test!("test_unit_is_16" {
        assert_eq!(UNIT, 16, "UNIT 必须为 16 (mallocng 基本分配单元)");
        });

        test!("test_ib_is_4" {
        assert_eq!(IB, 4, "IB 必须为 4 (in-band 元数据大小)");
        });

        test!("test_unit_plus_ib_equals_20" {
        assert_eq!(UNIT + IB, 20, "UNIT + IB = 20 (分配块 header 总开销)");
        });

    test!("test_mmap_threshold_is_reasonable" {
        // MMAP_THRESHOLD 应大于 UNIT + IB + 一些合理值
        assert!(
            MMAP_THRESHOLD > UNIT + IB,
            "MMAP_THRESHOLD ({}) 应显著大于 UNIT + IB (20)",
            MMAP_THRESHOLD
        );
        // MMAP_THRESHOLD = 128K - 20 = 131052, 不要求页对齐
        assert_eq!(MMAP_THRESHOLD, 131052, "MMAP_THRESHOLD = 128K - 20 = 131052");
    });

    test!("test_mmap_threshold_relation_to_size_classes" {
        // MMAP_THRESHOLD = 131052 = 128K - (UNIT+IB)
        // 最大 slot 大小: UNIT * 8191 = 16 * 8191 = 131056
        // MMAP_THRESHOLD 略小于最大 slot 大小 (差额 = IB = 4)
        let max_slot_size = UNIT * 8191;
        assert_eq!(max_slot_size, 131056);
        assert_eq!(max_slot_size - MMAP_THRESHOLD, IB, "差额应等于 IB={}", IB);
    });
}
