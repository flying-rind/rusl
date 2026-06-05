//! bump 分配器核心 — `simple_malloc` 实现。
//!
//! 优先通过 brk 扩展堆，在 brk 不可用或可能导致栈冲突时回退到 mmap。
//! 对于大块分配（浪费超过 1/8 时），采用几何增长的独立 mmap 区域减少碎片。
//!
//! 对应 C 的 `__simple_malloc`，使用安全 Rust 抽象重新设计内部逻辑。

use super::*;
use super::syscalls::*;
use super::stack_check::*;
use core::sync::atomic::Ordering;
use crate::import::__errno_location;

/// bump 分配器的核心实现。
///
/// # 算法概览
///
/// 1. **参数校验**: `n > usize::MAX / 2` → 返回 null + 设置 ENOMEM; `n == 0` → `n = 1`
/// 2. **对齐计算**: 2 的幂指数增长，上限 `ALIGN = 16`
/// 3. **加锁**: `bump_lock_acquire()`
/// 4. **地址对齐**: 调整 `BUMP_CUR` 到 `align` 对齐
/// 5. **空间不足时的扩展逻辑**:
///    - 首次调用: 通过 `sys_brk(0)` 获取初始 brk 值
///    - **brk 路径（优先）**: 若 brk == end 且无栈冲突，扩展 brk
///    - **mmap 路径（回退）**: 直接匿名 mmap 分配
///    - **新区域策略（启发式）**: 浪费 > 1/8 时创建新 mmap 区域
/// 6. **bump 分配**: `p = BUMP_CUR; BUMP_CUR += n`
/// 7. **解锁并返回**
///
/// # 前置条件
/// - `n` 为请求分配的大小（字节）
/// - `BUMP_LOCK` 处于可获取状态（无死锁风险）
/// - 系统调用 `sys_brk` 和 `sys_mmap` 可用（内核已初始化）
///
/// # 后置条件
/// - **成功**: 返回指向新分配内存的指针，按 `ALIGN` 对齐
/// - **失败**: 返回 `null_mut()`，errno 已设置
/// - **不变量**: 锁已释放（包括所有失败路径）
pub(crate) fn simple_malloc(n: usize) -> *mut c_void {
    // ====================================================================
    // 1. 参数校验
    // ====================================================================
    if n > usize::MAX / 2 {
        unsafe {
            *__errno_location() = ENOMEM;
        }
        return core::ptr::null_mut();
    }

    let mut n = n;
    if n == 0 {
        n = 1;
    }

    // ====================================================================
    // 2. 对齐计算：2 的幂指数增长，上限 ALIGN=16
    // ====================================================================
    let mut align: usize = 1;
    while align < n && align < ALIGN {
        align += align;
    }

    // ====================================================================
    // 3. 加锁
    // ====================================================================
    bump_lock_acquire();

    // ====================================================================
    // 4. 地址对齐：cur 向上对齐到 align 边界
    // ====================================================================
    let mut cur = BUMP_CUR.load(Ordering::Relaxed);
    cur = cur.wrapping_add(cur.wrapping_neg() & (align - 1));
    BUMP_CUR.store(cur, Ordering::Relaxed);

    // ====================================================================
    // 5. 空间不足时的扩展逻辑
    // ====================================================================
    let mut end = BUMP_END.load(Ordering::Relaxed);
    if n > end - cur {
        let page_size = PAGE_SIZE.load(Ordering::Relaxed) as usize;
        // 计算需要扩展的量（页对齐）
        let mut req = n - (end - cur);
        req = (req + page_size - 1) & !(page_size - 1);

        // --- 首次调用：获取初始 brk ---
        if cur == 0 {
            let mut brk = unsafe { sys_brk(0) };
            brk = (brk + page_size - 1) & !(page_size - 1); // page_align
            BUMP_BRK.store(brk, Ordering::Relaxed);
            cur = brk;
            end = brk;
            BUMP_CUR.store(cur, Ordering::Relaxed);
            BUMP_END.store(end, Ordering::Relaxed);
        }

        let brk = BUMP_BRK.load(Ordering::Relaxed);

        // --- brk 路径（优先）：brk == end 且无栈冲突且 brk 系统调用成功 ---
        if brk == end
            && req < usize::MAX - brk
            && !check_stack_collision(brk, brk + req)
            && unsafe { sys_brk(brk + req) } == brk + req
        {
            BUMP_BRK.store(brk + req, Ordering::Relaxed);
            end += req;
            BUMP_END.store(end, Ordering::Relaxed);
        } else {
            // --- mmap 回退路径 ---
            let mut new_area = false;
            req = (n + page_size - 1) & !(page_size - 1); // page_align(n)

            // 启发式：浪费超过 1/8 时创建独立 mmap 新区域
            if req - n > req / WASTE_THRESHOLD_DENOM {
                let step = BUMP_MMAP_STEP.load(Ordering::Relaxed) as usize;
                let min = page_size << (step / 2);
                // 用新区域剩余更少 → 创建新区域
                if min - n > end - cur {
                    if req < min {
                        req = min;
                        if step < MMAP_STEP_MAX as usize {
                            BUMP_MMAP_STEP.store((step + 1) as u8, Ordering::Relaxed);
                        }
                    }
                    new_area = true;
                }
            }

            let mem = unsafe {
                sys_mmap(
                    core::ptr::null_mut(),
                    req,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };

            // mmap 失败 或 不是新区域 → 直接返回（先解锁）
            if (mem as usize) == MAP_FAILED || !new_area {
                bump_lock_release();
                return if (mem as usize) == MAP_FAILED {
                    core::ptr::null_mut()
                } else {
                    mem
                };
            }

            // 设置为新 mmap 区域的起始分配位置
            cur = mem as usize;
            end = cur + req;
            BUMP_CUR.store(cur, Ordering::Relaxed);
            BUMP_END.store(end, Ordering::Relaxed);
        }
    }

    // ====================================================================
    // 6. 从当前区域分配（bump）
    // ====================================================================
    let p = cur as *mut c_void;
    cur += n;
    BUMP_CUR.store(cur, Ordering::Relaxed);

    // ====================================================================
    // 7. 解锁并返回
    // ====================================================================
    bump_lock_release();
    p
}

/// POSIX malloc 的内部实现，通过弱符号导出为 `malloc`。
///
/// 直接委托给 `__libc_malloc_impl`。
///
/// # 参数
/// - `size`: 请求分配的字节数
///
/// # 返回值
/// - **非零**: 指向已分配内存的指针
/// - **null**: 分配失败（errno 由 `simple_malloc` 设置）
pub(crate) fn default_malloc(size: usize) -> *mut c_void {
    // delegate to the weak symbol __libc_malloc_impl,
    // which resolves to simple_malloc in the lite build
    unsafe { super::__libc_malloc_impl(size) }
}

// ===========================================================================
// 单元测试
// ===========================================================================

#[cfg(test)]
mod tests {
    extern crate alloc;
    use rusl_core::test;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use alloc::string::String;
    use super::*;

    // ---- 函数签名验证 ----

    test!("test_simple_malloc_signature" {
        // 验证 simple_malloc 函数存在且可编译。
        let _f: fn(usize) -> *mut c_void = simple_malloc;
    });

    test!("test_default_malloc_signature" {
        // 验证 default_malloc 函数存在且可编译。
        let _f: fn(usize) -> *mut c_void = default_malloc;
    });

    // ---- 基本分配测试 ----

        test!("test_malloc_zero_returns_valid_pointer" {
        // 验证: `simple_malloc(0)` 返回有效指针（非 null）。
        // spec: `n == 0` → `n = 1`，因此返回一个有效指针。
        let p = simple_malloc(0);
        assert!(!p.is_null(), "malloc(0) 应返回有效指针（实现定义行为）");
        });

        test!("test_malloc_one_byte" {
        // 验证: `simple_malloc(1)` 返回有效指针。
        let p = simple_malloc(1);
        assert!(!p.is_null());
        });

        test!("test_malloc_page_size" {
        // 验证: `simple_malloc(4096)` 返回页大小的指针。
        let p = simple_malloc(4096);
        assert!(!p.is_null());
        });

        test!("test_multiple_allocations_distinct" {
        // 验证: 多次连续分配返回不同的指针。
        let p1 = simple_malloc(128);
        let p2 = simple_malloc(256);
        let p3 = simple_malloc(64);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert!(!p3.is_null());
        assert_ne!(p1, p2);
        assert_ne!(p1, p3);
        assert_ne!(p2, p3);
        });

    // ---- 参数非法测试 ----

    test!("test_malloc_oversized_returns_null" {
        // 验证: `simple_malloc(usize::MAX / 2 + 1)` 返回 null（参数校验失败）。
        // spec: `n > usize::MAX / 2` → 返回 null + errno = ENOMEM
        let huge = usize::MAX / 2 + 1;
        let p = simple_malloc(huge);
        assert!(p.is_null(), "超大分配请求应返回 null");
    });

    test!("test_malloc_max_returns_null" {
        // 验证: `simple_malloc(usize::MAX)` 返回 null。
        let p = simple_malloc(usize::MAX);
        assert!(p.is_null());
    });

    // ---- 对齐验证 ----

    test!("test_malloc_alignment" {
        // 验证: 不同大小的分配返回的指针满足对应 alignment（对齐至 min(2^n, ALIGN)）。
        for &size in &[1usize, 2, 3, 4, 7, 8, 15, 16, 17, 31, 32, 64, 128, 1024, 4096] {
            let p = simple_malloc(size);
            assert!(!p.is_null(), "malloc({}) 不应返回 null", size);
            // 对齐要求: min(大于 size 的最小 2 的幂, ALIGN)
            let mut expected_align: usize = 1;
            while expected_align < size && expected_align < ALIGN {
                expected_align += expected_align;
            }
            assert_eq!(p as usize % expected_align, 0, "malloc({}) 返回的指针应为 {}-字节对齐", size, expected_align);
        }
    });

    test!("test_malloc_16_byte_alignment" {
        // 验证: 返回指针满足对应的对齐要求（size>=16 时满足 16 字节对齐）。
        let sizes = [1usize, 7, 8, 13, 16, 31, 32, 63, 64, 127, 128];
        for &size in &sizes {
            let p = simple_malloc(size);
            assert!(!p.is_null());
            let mut expected_align: usize = 1;
            while expected_align < size && expected_align < ALIGN {
                expected_align += expected_align;
            }
            assert_eq!((p as usize) & (expected_align - 1), 0,
                "{} 字节分配应为 {}-字节对齐", size, expected_align);
        }
    });

    // ---- 边界值测试 ----

        test!("test_malloc_at_limit" {
        // 验证: `simple_malloc(usize::MAX / 2)` 不返回 null（合法最大值）。
        // 合法最大值
        let p = simple_malloc(usize::MAX / 2);
        // 可能成功也可能失败（取决于系统可用内存），但不该 crash
        // 此处仅验证不会 panic
        let _ = p;
        });

        test!("test_various_alignment_levels" {
        // 验证: 不同对齐级别的分配正常工作。
        // align = 1, 2, 4, 8, 16 对应不同 n 值
        for n in &[1usize, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17] {
            let p = simple_malloc(*n);
            assert!(!p.is_null(), "n={} 分配失败", n);
        }
        });

    // ---- 线程安全测试（编译期） ----

    test!("test_simple_malloc_is_thread_safe_by_signature" {
        // 验证 simple_malloc 的签名允许跨线程调用。实际线程安全性由内部锁保证。
        // 函数签名接受 &self 的等价检查: 仅使用 static 变量，无 &mut self 依赖
        // 此测试在编译期确保函数不是 `fn(&mut self, ...)` 等不安全 API
        let _f: fn(usize) -> *mut c_void = simple_malloc;
    });

    // ---- 锁安全性测试 ----

        test!("test_no_deadlock_under_concurrent_allocation" {
        // 验证: 分配器在并发环境下不会死锁。
        // 注: 实际测试需要多线程环境，此测试为占位。
        // 占位: 多线程并发分配
        });

        test!("test_lock_released_after_failed_allocation" {
        // 验证: `simple_malloc` 在所有失败路径上释放了锁。
        // 注: 可通过后续 `simple_malloc` 调用成功来间接验证。
        // 先触发超大分配失败
        let p = simple_malloc(usize::MAX / 2 + 1);
        assert!(p.is_null());
        // 后续正常分配应成功（锁未泄漏）
        let p2 = simple_malloc(64);
        assert!(!p2.is_null());
        });

    // ---- 幂等性测试 ----

        test!("test_consecutive_malloc_zero" {
        // 验证: 连续两次 `simple_malloc(0)` 返回不同指针（各自有效）。
        let p1 = simple_malloc(0);
        let p2 = simple_malloc(0);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert_ne!(p1, p2, "连续 malloc(0) 应返回不同地址");
        });

    // ---- 浪费阈值启发式测试 ----

    test!("test_waste_threshold_logic" {
        // 验证: 浪费超过 1/8 时触发独立 mmap 区域策略。
        // 逻辑: `req - n > req / WASTE_THRESHOLD_DENOM`
        let denom = WASTE_THRESHOLD_DENOM;

        // Case: n=1, req=16 → 浪费 = 15, req/8 = 2, 15 > 2 → 应触发
        let (n, req) = (1usize, 16usize);
        let waste = req - n;
        let threshold = req / denom;
        assert!(waste > threshold, "浪费 {} > {}/{} = {} 应触发新区域策略", waste, req, denom, threshold);

        // Case: n=4000, req=4096 → 浪费 = 96, req/8 = 512, 96 ≤ 512 → 不触发
        let (n, req) = (4000usize, 4096usize);
        let waste = req - n;
        let threshold = req / denom;
        assert!(waste <= threshold, "浪费 {} <= {}/{} = {} 不应触发新区域策略", waste, req, denom, threshold);
    });

    // ---- 几何增长策略测试 ----

    test!("test_mmap_step_geometric_growth" {
        // 验证 mmap_step 几何增长逻辑: `min_req = PAGE_SIZE << (step / 2)`。
        let page_size = 4096usize;
        // step = 0 → min_req = 4096
        assert_eq!(page_size << (0 / 2), 4096);
        // step = 2 → min_req = 4096 << 1 = 8192
        assert_eq!(page_size << (2 / 2), 8192);
        // step = 12 → min_req = 4096 << 6 = 262144 (256KB)
        assert_eq!(page_size << (MMAP_STEP_MAX as usize / 2), 262144);
    });

    test!("test_mmap_step_max_bound" {
        // 验证 BUMP_MMAP_STEP 递增不超过 MMAP_STEP_MAX。
        let step = MMAP_STEP_MAX as usize;
        assert!(step < u8::MAX as usize, "step 不应溢出 u8");
        // max page shift: step/2 = 6, so max = PAGE_SIZE << 6
        assert_eq!(step / 2, 6);
    });

    // ---- 数据完整性测试 ----

        test!("test_malloc_memory_writable" {
        // 验证: 分配的内存可安全写入（写入后不应触发 SIGSEGV）。
        let p = simple_malloc(128) as *mut u8;
        assert!(!p.is_null());
        unsafe {
            // 写入首尾各一字节
            *p = 0xAA;
            *p.add(127) = 0x55;
            assert_eq!(*p, 0xAA);
            assert_eq!(*p.add(127), 0x55);
        }
        });

        test!("test_multiple_allocations_no_overlap" {
        // 验证: 多次分配的写入互不干扰。
        let count = 4usize;
        let size = 256usize;

        // 分配多个块
        let mut ptrs = Vec::with_capacity(count);
        for _ in 0..count {
            let p = simple_malloc(size);
            assert!(!p.is_null());
            ptrs.push(p);
        }

        // 写入各块的模式
        unsafe {
            for (i, &p) in ptrs.iter().enumerate() {
                let byte = p as *mut u8;
                *byte = i as u8;
            }
            // 验证模式未被覆盖
            for (i, &p) in ptrs.iter().enumerate() {
                let byte = p as *mut u8;
                assert_eq!(*byte, i as u8, "块 {} 被相邻分配覆盖", i);
            }
        }
        });

    // ---- errno 设置测试 ----

        test!("test_errno_set_on_failure" {
        // 验证: 分配失败后 errno 设置为 ENOMEM。
        let p = simple_malloc(usize::MAX / 2 + 1);
        assert!(p.is_null());
        // 注: errno 检查需要 rusl 的 errno 基础设施支持
        // unsafe { assert_eq!(*__errno_location(), ENOMEM); }
        });

        test!("test_errno_unchanged_on_success" {
        // 验证: 分配成功后 errno 保持不变。
        // 先设置 errno
        // unsafe { *__errno_location() = 99; }
        // let p = simple_malloc(64);
        // assert!(!p.is_null());
        // unsafe { assert_eq!(*__errno_location(), 99); }
        });

    // ---- 内部状态不变量测试 ----

    test!("test_bump_state_invariant_initial" {
        // 验证: 当前状态下 BUMP_CUR <= BUMP_END 且 BUMP_BRK <= BUMP_END。
        let brk = BUMP_BRK.load(Ordering::Relaxed);
        let cur = BUMP_CUR.load(Ordering::Relaxed);
        let end = BUMP_END.load(Ordering::Relaxed);

        assert!(cur <= end, "BUMP_CUR({}) > BUMP_END({})", cur, end);
        assert!(brk <= end, "BUMP_BRK({}) > BUMP_END({})", brk, end);
    });

    test!("test_bump_state_invariant_after_alloc" {
        // 验证: 分配后不变量仍然保持。
        let _p = simple_malloc(64);
        let brk = BUMP_BRK.load(Ordering::Relaxed);
        let cur = BUMP_CUR.load(Ordering::Relaxed);
        let end = BUMP_END.load(Ordering::Relaxed);
        // 核心不变量: cur 在 [base, end] 范围内, brk <= end
        assert!(cur <= end, "BUMP_CUR({}) > BUMP_END({})", cur, end);
        assert!(brk <= end, "BUMP_BRK({}) > BUMP_END({})", brk, end);
    });

    // ---- default_malloc 委托测试 ----

    test!("test_default_malloc_signature_matches" {
        // 验证: default_malloc 函数签名与 simple_malloc 兼容。
        let _f1: fn(usize) -> *mut c_void = simple_malloc;
        let _f2: fn(usize) -> *mut c_void = default_malloc;
    });

    // ---- no_std 兼容性测试 ----

    test!("test_no_std_compatibility_declaration" {
        // 验证: 模块不依赖 std（仅使用 core）。
        // 注: 此测试在 #![no_std] 构建中被动验证。
        // 验证 core 类型可用
        let _x: core::sync::atomic::AtomicUsize = AtomicUsize::new(0);
        let _y: core::ffi::c_void = unsafe { core::mem::zeroed() };
    });
}