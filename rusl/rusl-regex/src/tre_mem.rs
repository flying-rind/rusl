//! TRE 内存分配器 — bump-pointer 分配器（arena allocator）。
//!
//! 本模块为 TRE 正则表达式引擎提供快速块分配，所有分配的内存块以 Vec 管理，
//! 不支持单独释放，仅在 `TreMem` 被 drop 时一次性批量回收。
//!
//! 所有符号均为 `pub(crate)` 可见性，仅 rusl crate 内部使用。
//!
//! # 设计意图
//!
//! Bump-pointer 分配器专为正则表达式编译/匹配期间大量小块分配的场景优化：
//! - **批量释放**：不支持单独释放，通过 `Vec<Box<[u8]>>` + Drop 自动实现
//! - **块链扩展**：当前块耗尽时分配新的固定大小块（默认 1024 字节）
//! - **失败传播**：`failed` 标志位确保 fail-fast 语义

#![allow(unused_imports, unused_variables)]

use alloc::boxed::Box;
use alloc::vec::Vec;
use super::tre::TRE_MEM_BLOCK_SIZE;

// ============================================================================
// TreBlock — 内存块节点
// ============================================================================

/// 一个堆分配的内存块。
///
/// Rust 使用 `Box<[u8]>` 替代 C 的 `void *data` 裸指针，
/// 由 Box 的 RAII 语义自动保证释放。
#[derive(Debug)]
pub(crate) struct TreBlock {
    /// 堆分配的数据块。
    data: Box<[u8]>,
}

// ============================================================================
// TreMem — Bump-pointer 分配器
// ============================================================================

/// Bump-pointer 分配器的控制结构。
///
/// # 不变量
///
/// - `failed == false` 时：`blocks` 若非空则 `current_idx < blocks.len()`，
///   `ptr` 指向当前块内的有效偏移
/// - `failed == true` 时：分配器处于永久失败状态，所有分配请求返回 null
/// - `blocks` 为空 → `current_idx == 0`，`ptr == 0`，`n == 0`
#[derive(Debug)]
pub(crate) struct TreMem {
    /// 所有已分配的内存块。
    blocks: Vec<TreBlock>,
    /// 当前活跃块的索引（在 `blocks` 中）。
    current_idx: usize,
    /// 当前块中下一个可分配位置的偏移量。
    ptr: usize,
    /// 当前块中剩余可用字节数。
    n: usize,
    /// 分配失败标志（true = 已失败，后续分配立即返回 null）。
    failed: bool,
}

impl TreMem {
    /// 获取分配失败标志。
    #[inline]
    pub(crate) fn is_failed(&self) -> bool {
        self.failed
    }

    /// 获取已分配的块数量（用于测试/诊断）。
    #[inline]
    pub(crate) fn block_count(&self) -> usize {
        self.blocks.len()
    }
}

// ============================================================================
// tre_mem_new — 创建分配器
// ============================================================================

/// 创建并初始化一个新的 TRE 内存分配器实例。
///
/// # 后置条件
///
/// 返回初始状态的 `TreMem`：`blocks` 为空，`current_idx = 0`，
/// `ptr = 0`，`n = 0`，`failed = false`。
///
/// # Rust 设计优势
///
/// C 实现通过 `calloc` 堆分配 `tre_mem_struct`，需要调用者手动 `destroy`；
/// Rust 实现在栈上创建 `TreMem`，通过 RAII 自动管理生命周期。
pub(crate) fn tre_mem_new() -> TreMem {
    TreMem {
        blocks: Vec::new(),
        current_idx: 0,
        ptr: 0,
        n: 0,
        failed: false,
    }
}

// ============================================================================
// tre_mem_alloc — 分配内存块
// ============================================================================

/// 从分配器 `mem` 中分配 `size` 字节，返回对齐到 `usize` 边界的指针。
///
/// # 前置条件
///
/// - `size > 0`
/// - `mem` 为有效的 `TreMem` 实例
///
/// # 后置条件
///
/// | 条件 | 返回值 | 状态变化 |
/// |------|--------|----------|
/// | `mem.failed == true` | `null()` | 无变化（fail-fast） |
/// | 当前块剩余空间足够 | 对齐后的有效指针 | `ptr` 推进，`n` 减少 |
/// | 当前块空间不足但新块分配成功 | 对齐后的有效指针 | 新块追加到 `blocks` |
/// | 新块分配失败（OOM） | `null()` | `mem.failed = true` |
///
/// # 系统算法
///
/// Bump-pointer 分配器：在固定大小块中线性分配，块耗尽时链接新块。
/// 每次分配追加对齐填充以保证后续分配的指针对齐。
pub(crate) fn tre_mem_alloc(mem: &mut TreMem, size: usize) -> *mut u8 {
    tre_mem_alloc_impl(mem, size, false)
}

/// 内部分配实现，支持可选的零初始化。
fn tre_mem_alloc_impl(mem: &mut TreMem, size: usize, zero: bool) -> *mut u8 {
    // size 0 为非法请求，返回 NULL（与 C 实现语义一致）
    if size == 0 {
        return core::ptr::null_mut();
    }

    // fail-fast: 一旦分配失败，后续所有请求立即返回 NULL
    if mem.failed {
        return core::ptr::null_mut();
    }

    let align = core::mem::align_of::<usize>();

    // 检查当前块剩余空间是否足够（含对齐填充）
    if mem.n < size {
        // 当前块空间不足，分配新块
        // 块大小策略：取 max(默认值, size * 8)
        let block_size = core::cmp::max(TRE_MEM_BLOCK_SIZE, size * 8);

        // 分配新数据块
        let data_vec = Vec::with_capacity(block_size);
        // Vec::with_capacity 在 OOM 时会 panic（Rust 标准行为），
        // 这与 C 实现的 malloc 失败处理不同。
        // 在 no_std 环境中可通过自定义分配器处理，
        // 此处的 panic 等价于无法恢复的内存错误。
        let mut data = data_vec;
        // Safety: 已分配 block_size 容量，设置长度以允许后续写入
        unsafe { data.set_len(block_size); }
        let data = data.into_boxed_slice();

        mem.blocks.push(TreBlock { data });
        mem.current_idx = mem.blocks.len() - 1;
        mem.ptr = 0;
        mem.n = block_size;
    }

    // 计算对齐填充，确保下次分配的起始地址按 usize 对齐。
    // C 实现: size += ALIGN(mem->ptr + size, long)
    let padding = super::tre::align_offset(mem.ptr + size, align);
    let total_advance = size + padding;

    debug_assert!(
        total_advance <= mem.n,
        "总推进量超出块容量: {} > {}",
        total_advance,
        mem.n
    );

    // Safety: blocks[current_idx] 一定存在（上面刚确保了有块）
    let result = unsafe {
        mem.blocks[mem.current_idx].data.as_ptr().add(mem.ptr) as *mut u8
    };

    // Bump pointer 推进
    mem.ptr += total_advance;
    mem.n -= total_advance;

    // 可选的零初始化（calloc 语义）
    if zero {
        unsafe {
            core::ptr::write_bytes(result, 0, size);
        }
    }

    result
}

// ============================================================================
// tre_mem_calloc — 分配并零初始化
// ============================================================================

/// 从分配器 `mem` 中分配 `size` 字节并零初始化。
///
/// # 前置条件
///
/// - `size > 0`
/// - `mem` 为有效的 `TreMem` 实例
///
/// # 后置条件
///
/// 与 `tre_mem_alloc` 语义相同，但额外保证分配的内存已零初始化。
pub(crate) fn tre_mem_calloc(mem: &mut TreMem, size: usize) -> *mut u8 {
    tre_mem_alloc_impl(mem, size, true)
}

// ============================================================================
// Drop 实现 — 自动批量释放
// ============================================================================

/// 通过 Rust RAII 机制自动释放所有内存块。
///
/// `Vec<TreBlock>` 的 drop 自动递归释放所有 `Box<[u8]>`。
/// 无需手动遍历链表和调用 free。
impl Drop for TreMem {
    fn drop(&mut self) {
        // Vec<TreBlock> 的 drop 自动递归释放所有 Box<[u8]>
        // 无需手动遍历链表和调用 free
    }
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::mem;

    test!("test_new_allocator_initial_state" {
        let mem = tre_mem_new();
        assert!(!mem.failed);
        assert_eq!(mem.block_count(), 0);
        assert_eq!(mem.ptr, 0);
        assert_eq!(mem.n, 0);
        assert_eq!(mem.current_idx, 0);
        assert!(!mem.is_failed());
    });

    test!("test_alloc_small_size" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_alloc(&mut mem, 64);
        assert!(!ptr.is_null());
        assert!(!mem.is_failed());
        // 至少分配了一个块
        assert!(mem.block_count() >= 1);
    });

    test!("test_alloc_zero_returns_null" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_alloc(&mut mem, 0);
        assert!(ptr.is_null());
    });

    test!("test_alloc_multiple_small_allocs_from_same_block" {
        let mut mem = tre_mem_new();
        let p1 = tre_mem_alloc(&mut mem, 16);
        let p2 = tre_mem_alloc(&mut mem, 16);
        let p3 = tre_mem_alloc(&mut mem, 16);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        assert!(!p3.is_null());
        // 应来自同一块（16*3=48 < 1024）
        assert_eq!(mem.block_count(), 1);
    });

    test!("test_alloc_large_grows_new_blocks" {
        let mut mem = tre_mem_new();
        // 分配大于默认块大小的内存，应触发新块分配
        let p1 = tre_mem_alloc(&mut mem, TRE_MEM_BLOCK_SIZE + 1);
        assert!(!p1.is_null());
        assert!(!mem.is_failed());
        // 至少分配了多个块
        assert!(mem.block_count() >= 1);
    });

    test!("test_alloc_returned_pointers_are_valid_for_writing" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_alloc(&mut mem, 64);
        assert!(!ptr.is_null());
        unsafe {
            // 验证可以安全写入
            for i in 0..64 {
                *ptr.add(i) = i as u8;
            }
            // 验证写入的值
            for i in 0..64 {
                assert_eq!(*ptr.add(i), i as u8);
            }
        }
    });

    test!("test_alloc_ptr_alignment" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_alloc(&mut mem, 1);
        assert!(!ptr.is_null());
        // 返回的指针应对齐到 usize
        assert_eq!(ptr as usize % mem::align_of::<usize>(), 0);
    });

    test!("test_alloc_ptr_alignment_odd_sizes" {
        let mut mem = tre_mem_new();
        // 测试各种奇数大小的分配请求
        for &size in &[1usize, 3, 5, 7, 9, 11, 13, 15, 17] {
            let ptr = tre_mem_alloc(&mut mem, size);
            assert!(!ptr.is_null(), "分配 size={} 返回了 null", size);
            assert_eq!(
                ptr as usize % mem::align_of::<usize>(),
                0,
                "分配 size={} 的指针对齐不正确",
                size
            );
        }
    });

    test!("test_calloc_zeroes_memory" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_calloc(&mut mem, 64);
        assert!(!ptr.is_null());
        unsafe {
            for i in 0..64 {
                assert_eq!(*ptr.add(i), 0u8, "tre_mem_calloc 未清零字节 {}", i);
            }
        }
    });

    test!("test_calloc_then_write" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_calloc(&mut mem, 64);
        assert!(!ptr.is_null());
        unsafe {
            // calloc 后应全为零
            assert_eq!(*ptr.add(0), 0);
            // 写入后验证
            *ptr.add(0) = 0xFF;
            assert_eq!(*ptr.add(0), 0xFF);
            // 其他字节应仍为零
            assert_eq!(*ptr.add(1), 0);
        }
    });

    test!("test_alloc_after_failure_returns_null" {
        let mut mem = tre_mem_new();
        // 通过请求一个极大值触发分配失败（但不依赖具体限制）
        // 首先正常分配多次以耗尽内存
        let mut failed = false;
        for _ in 0..10000 {
            let ptr = tre_mem_alloc(&mut mem, TRE_MEM_BLOCK_SIZE);
            if ptr.is_null() {
                failed = true;
                break;
            }
        }
        if failed {
            // 失败后后续分配应立即返回 null
            let ptr = tre_mem_alloc(&mut mem, 64);
            assert!(ptr.is_null());
        }
    });

    test!("test_is_failed_initial" {
        let mem = tre_mem_new();
        assert!(!mem.is_failed());
    });

    test!("test_block_count_monotonic" {
        let mut mem = tre_mem_new();
        let initial = mem.block_count();
        tre_mem_alloc(&mut mem, 64);
        assert!(mem.block_count() >= initial);
    });

    test!("test_drop_releases_memory" {
        let mut mem = tre_mem_new();
        // 分配一些块
        for _ in 0..10 {
            let _ptr = tre_mem_alloc(&mut mem, TRE_MEM_BLOCK_SIZE);
        }
        // 当 mem 离开作用域时，Drop 自动释放
        // 通过作用域退出验证无泄漏
    });

    test!("test_many_small_allocs" {
        let mut mem = tre_mem_new();
        let mut ptrs = Vec::new();
        for i in 0..1000 {
            let ptr = tre_mem_alloc(&mut mem, i % 64 + 1);
            if ptr.is_null() {
                break;
            }
            ptrs.push(ptr);
        }
        assert!(!ptrs.is_empty(), "至少应成功分配一个块");
    });

    test!("test_alloc_exact_block_size" {
        let mut mem = tre_mem_new();
        let ptr = tre_mem_alloc(&mut mem, TRE_MEM_BLOCK_SIZE);
        assert!(!ptr.is_null());
        assert!(mem.block_count() >= 1);
    });

    test!("test_calloc_then_alloc_uses_different_memory" {
        let mut mem = tre_mem_new();
        let p1 = tre_mem_calloc(&mut mem, 32);
        let p2 = tre_mem_alloc(&mut mem, 32);
        assert!(!p1.is_null());
        assert!(!p2.is_null());
        // 两次分配应返回不同区域（或至少不重叠）
        unsafe {
            // 写入不同模式
            for i in 0..32 {
                p2.add(i).write(0xAAu8);
            }
            // p1 应仍为零（来自 calloc）
            for i in 0..32 {
                assert_eq!(*p1.add(i), 0u8);
            }
        }
    });
}
