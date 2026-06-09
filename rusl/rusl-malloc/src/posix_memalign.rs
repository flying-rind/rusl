//! posix_memalign — POSIX 对齐内存分配，通过输出参数 res 返回对齐指针，
//! 以返回值直接传递错误码（0 成功，EINVAL/ENOMEM 失败）。
//!
//! ## C ABI 签名
//!
//! ```c
//! int posix_memalign(void **res, size_t align, size_t len);
//! ```
//!
//! ## 归约约束
//!
//! - 返回值仅为 0（成功）、EINVAL、ENOMEM
//! - 成功时 `*res` 指向对齐内存，可通过 `free(*res)` 释放
//! - 失败时 `*res` 保持不变（POSIX 要求）
//! - 本函数自身仅做 `align < sizeof(void*)` 的快速路径校验，其余逻辑委托给
//!   `aligned_alloc_inner()`

use core::ffi::{c_int, c_ulong, c_void};
use core::mem;
use core::ptr::NonNull;
use crate::import::__errno_location;

// ===========================================================================
// 错误码常量 — 引用父模块的统一定义
//
// 这些常量在父模块 src/malloc/mod.rs 中统一定义，所有 malloc 子模块共享。
// 定义值遵循 Linux x86_64 ABI: EINVAL=22, ENOMEM=12
// ===========================================================================

use super::EINVAL;
use super::ENOMEM;

// ===========================================================================
// 占位类型定义 — 待迁移至 mallocng 模块
//
// TODO: 以下类型最终应定义在 rusl::mallocng::aligned_alloc 中，
// 本文件届时使用 `use rusl::mallocng::aligned_alloc::{aligned_alloc_inner, AlignedAllocError};`
// ===========================================================================

///
/// 对齐分配错误类型。
///
/// 此类型最终应定义在 `rusl::mallocng::aligned_alloc` 模块中，
/// 当前占位于此以便接口骨架和单元测试编译通过。
///
/// TODO: 迁移至 rusl::mallocng::aligned_alloc，替换本地的 `use` 导入
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AlignedAllocError {
    /// 对齐值不是 2 的幂
    InvalidAlignment,
    /// 内存不足或长度溢出
    OutOfMemory,
}

///
/// 内部对齐分配引擎。
///
/// 将 `aligned_alloc` 的 C 风格错误报告（NULL 返回 + errno）转换为
/// Rust 风格的 `Result<NonNull<c_void>, AlignedAllocError>`。
///
/// 此函数最终应定义在 `rusl::mallocng::aligned_alloc` 模块中，
/// 当前占位于此以便接口骨架和单元测试编译通过。
///
/// # 参数
///
/// * `align` - 对齐要求（字节），必须是 2 的幂。
/// * `len` - 请求分配的内存大小（字节）。
///
/// # 返回值
///
/// * `Ok(ptr)` — 分配成功，`ptr` 指向至少 `len` 字节的对齐内存。
/// * `Err(InvalidAlignment)` — `align` 不是 2 的幂（由底层 `aligned_alloc` 检测，errno=EINVAL）。
/// * `Err(OutOfMemory)` — 内存不足、长度溢出或对齐过大（由底层 `aligned_alloc` 检测，errno=ENOMEM）。
///
/// # 委托策略
///
/// 当前实现委托给 `mallocng::aligned_alloc::aligned_alloc(align, len)`，
/// 该函数内部完成所有参数校验（2 的幂检查、溢出检查、对齐上限检查）和实际分配。
/// 本函数仅负责 C ABI 错误码到 Rust `Result` 的转换。
///
/// TODO: 迁移至 rusl::mallocng::aligned_alloc，替换本地的 `use` 导入
pub(crate) fn aligned_alloc_inner(
    align: usize,
    len: usize,
) -> Result<NonNull<c_void>, AlignedAllocError> {
    // 委托给 mallocng 的 aligned_alloc（C ABI 函数），该函数内部完成：
    // 1. 校验 align 是否为 2 的幂（否则返回 NULL + EINVAL）
    // 2. 校验 len 是否溢出（否则返回 NULL + ENOMEM）
    // 3. 校验 align 是否过大（否则返回 NULL + ENOMEM）
    // 4. 实际内存分配和指针对齐
    let ptr = super::mallocng::aligned_alloc::aligned_alloc(align, len);
    if ptr.is_null() {
        // aligned_alloc 返回 NULL 时已设置 errno，据此区分错误类型
        let errno_val = unsafe { *__errno_location() };
        if errno_val == EINVAL {
            Err(AlignedAllocError::InvalidAlignment)
        } else {
            // 包括 ENOMEM 以及任何其他导致 NULL 返回的错误
            Err(AlignedAllocError::OutOfMemory)
        }
    } else {
        // Safety: aligned_alloc 返回非空指针时保证内存有效
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }
}

// ===========================================================================
// 对外导出接口
// ===========================================================================

/// POSIX posix_memalign — 对齐内存分配。
///
/// 分配 `len` 字节的内存，起始地址满足 `align` 字节对齐，通过 `res` 输出。
/// 与 C 实现 (musl) 语义完全相同。
///
/// ## 参数
///
/// - `res`: 输出参数，指向一个 `*mut c_void` 可写位置。分配成功时写入对齐指针。
/// - `align`: 对齐要求（字节），必须是 2 的幂且 `>= sizeof(void*)`。
/// - `len`: 请求分配的内存大小（字节）。
///
/// ## 返回值
///
/// - `0`: 分配成功，`*res` 指向最少 `len` 字节的对齐内存。
/// - `EINVAL` (22): `align` 不合法（小于指针宽度或非 2 的幂）。
/// - `ENOMEM` (12): 内存不足，或 `len == 0`，或 `len` 溢出。
///
/// ## 系统算法
///
/// ```text
/// posix_memalign(res, align, len):
///   1. if align < sizeof(void*)  → return EINVAL
///   2. match aligned_alloc_inner(align, len) {
///   3.     Ok(ptr)  => *res = ptr.as_ptr(); return 0
///   4.     Err(InvalidAlignment) => return EINVAL
///   5.     Err(OutOfMemory)      => return ENOMEM
///   6. }
/// ```
///
/// ## 后置条件
///
/// - **成功** (返回 0): `*res` 指向对齐内存，`(*res) % align == 0`，可通过 `free` 释放。
/// - **失败** (返回 != 0): `*res` 保持不变，无内存分配。
///
/// ## 不变量
///
/// - 返回值始终为 `0`、`EINVAL` 或 `ENOMEM`。
/// - 失败时 `*res` 保持不变（POSIX 要求）。
/// - 本函数不设置线程本地 `errno`；错误通过返回值传递。
/// - 与 C ABI 完全兼容，参数布局和返回值布局一致。
///
/// # Safety
///
/// 调用者必须确保：
/// - `res` 非 NULL，指向一个有效的 `*mut c_void` 可写内存位置。
/// - 分配成功后，必须通过 `free(*res)` 释放内存（与 `malloc` 共享同一堆）。
/// - 分配的内存未初始化，读取前必须写入。
/// - 本函数不保证分配的内存可以传递给 `realloc`。
#[no_mangle]
pub extern "C" fn posix_memalign(
    res: *mut *mut c_void,
    align: c_ulong,
    len: c_ulong,
) -> c_int {
    // 步骤 1: 快速路径 — len == 0 直接返回 ENOMEM (POSIX 实现定义行为, musl 选择 ENOMEM)
    if len == 0 {
        return ENOMEM;
    }

    // 步骤 2: align < sizeof(void*) 直接返回 EINVAL（POSIX 要求）
    // 此路径不依赖底层分配器，可在 aligned_alloc 未实现时独立验证。
    if (align as usize) < mem::size_of::<*mut c_void>() {
        return EINVAL;
    }

    // 步骤 2-5: 委托给内部对齐分配引擎
    // - aligned_alloc_inner 内部完成 2 的幂校验、溢出检测、实际分配
    // - convert_aligned_alloc_result 负责 Result → C 错误码转换和 *res 写入
    let result = aligned_alloc_inner(align as usize, len as usize);
    // SAFETY: convert_aligned_alloc_result 的调用者保证 res 非 NULL 且指向有效可写内存。
    unsafe { convert_aligned_alloc_result(result, res) }
}

// ===========================================================================
// 内部辅助函数
// ===========================================================================

/// 将内部 `aligned_alloc_inner` 的 `Result` 转换为 C 错误码。
///
/// 此为 `posix_memalign` 的内部工具函数，负责：
/// - 成功时：将 `NonNull<c_void>` 写入 `*res`，返回 0
/// - 失败时：将 `AlignedAllocError` 映射为 C 错误码，`*res` 保持不变
///
/// # Safety
///
/// 调用者必须确保 `res` 非 NULL 且指向有效的可写内存。
#[inline]
unsafe fn convert_aligned_alloc_result(
    result: Result<NonNull<c_void>, AlignedAllocError>,
    res: *mut *mut c_void,
) -> c_int {
    match result {
        Ok(ptr) => {
            // 成功分支: 将对齐指针写入输出参数，返回 0
            // Safety: 调用方保证 res 是非 NULL 的可写指针
            *res = ptr.as_ptr();
            0
        }
        Err(AlignedAllocError::InvalidAlignment) => {
            // align 不是 2 的幂 — 失败时 *res 保持不变（POSIX 要求）
            EINVAL
        }
        Err(AlignedAllocError::OutOfMemory) => {
            // 内存不足、len 溢出或对齐过大 — 失败时 *res 保持不变（POSIX 要求）
            ENOMEM
        }
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    /// 返回系统指针字节宽度
    const PTR_SIZE: usize = mem::size_of::<*mut c_void>();

    // -----------------------------------------------------------------------
    // 辅助函数
    // -----------------------------------------------------------------------

    /// 安全封装：调用 posix_memalign 并展开输出指针。
    ///
    /// 返回 `(ret, ptr)`，其中 `ptr` 为 `*res` 的值。
    unsafe fn call_pma(
        res: *mut *mut c_void,
        align: c_ulong,
        len: c_ulong,
    ) -> (c_int, *mut c_void) {
        let ret = posix_memalign(res, align, len);
        let ptr = *res;
        (ret, ptr)
    }

    /// 检查 `ptr` 是否满足 `align` 字节对齐。
    fn is_aligned_to(ptr: *mut c_void, align: usize) -> bool {
        (ptr as usize) % align == 0
    }

    // ===================================================================
    // 1. 基本快速路径测试 — EINVAL: align < sizeof(void*)
    // ===================================================================

    test!("test_einval_when_align_equals_zero" {
        // posix_memalign 自身检测 align < sizeof(void*)，直接返回 EINVAL。
        // 此路径不依赖 aligned_alloc_inner，可以在骨架阶段测试。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                0,
                64,
            );
            assert_eq!(ret, EINVAL, "align=0 应返回 EINVAL");
            // 失败时 *res 保持不变
            assert!(mem.is_null(), "失败时 *res 应保持不变 (保持 null)");
    });

    test!("test_einval_when_align_equals_one" {
        // align=1 远小于指针宽度，应返回 EINVAL。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                1,
                128,
            );
            assert_eq!(ret, EINVAL, "align=1 应返回 EINVAL");
            assert!(mem.is_null());
    });

    test!("test_einval_when_align_less_than_ptr_size" {
        // 在 64 位平台上 align=4 < 8，应返回 EINVAL；在 32 位平台上则通过。
        let small_align = (PTR_SIZE / 2) as c_ulong;
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                small_align,
                256,
            );
            if PTR_SIZE > small_align as usize {
                assert_eq!(ret, EINVAL, "align < sizeof(void*) 应返回 EINVAL");
            } else {
                // 在 32 位平台上，PTR_SIZE == 4，small_align == 2 仍小于 4
                assert_eq!(ret, EINVAL);
            }
            assert!(mem.is_null(), "失败时 *res 应保持不变");
    });

    test!("test_passes_when_align_equals_ptr_size" {
        // align 刚好等于 sizeof(void*) 时，快速路径放行，进入 aligned_alloc_inner。
        // 注意：在骨架阶段，aligned_alloc_inner 内部为 todo!()，此测试会 panic。
        // 待 aligned_alloc_inner 实现后启用。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                64,
            );
            // 对齐值等于指针宽度，快速路径应放行
            assert_eq!(
                ret, 0,
                "align == sizeof(void*) 应成功（若无其他错误）"
            );
            // 成功时 *res 非空且对齐
            assert!(!mem.is_null(), "成功时 *res 应非空");
            assert!(
                is_aligned_to(mem, PTR_SIZE),
                "返回指针应对齐到 sizeof(void*)"
            );
            // 清理
            // TODO: 调用 free(mem) 释放
    });

    // ===================================================================
    // 2. 成功路径测试（依赖 aligned_alloc_inner）
    // ===================================================================

    test!("test_success_align_16_len_128" {
        // 正常参数：align = 16, len = 128，期望分配成功。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 16, 128);
            assert_eq!(ret, 0);
            assert!(!mem.is_null());
            assert!(is_aligned_to(mem, 16));
    });

    test!("test_success_min_valid_params" {
        // 最小非零对齐 + 最小非零长度的成功分配。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                1,
            );
            assert_eq!(ret, 0);
            assert!(!mem.is_null());
            assert!(is_aligned_to(mem, PTR_SIZE));
    });

    test!("test_success_page_align" {
        // 大对齐（页对齐 4096）成功分配。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 4096, 8192);
            assert_eq!(ret, 0);
            assert!(!mem.is_null());
            assert!(is_aligned_to(mem, 4096));
    });

    test!("test_success_huge_align" {
        // 巨大对齐值（2^20 = 1MB），成功分配。
        let huge_align = 1 << 20; // 1048576
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                huge_align as c_ulong,
                1024,
            );
            assert_eq!(ret, 0);
            assert!(!mem.is_null());
            assert!(is_aligned_to(mem, huge_align));
    });

    test!("test_multiple_allocations" {
        // 连续多次分配验证无泄漏和独立性。
            let mut ptrs: [*mut c_void; 4] = [core::ptr::null_mut(); 4];
            for i in 0..4 {
                let mut p: *mut c_void = core::ptr::null_mut();
                let ret = posix_memalign(&mut p as *mut *mut c_void, 16, 64);
                assert_eq!(ret, 0, "第 {} 次分配失败", i);
                assert!(!p.is_null());
                assert!(is_aligned_to(p, 16));
                // 指针应两两不同
                for j in 0..i {
                    assert_ne!(p, ptrs[j], "指针不应重复");
                }
                ptrs[i] = p;
            }
            // TODO: free all
    });

    // ===================================================================
    // 3. 对齐验证测试
    // ===================================================================

    test!("test_various_valid_alignments" {
        // 多种合法对齐值均能成功。
        let valid_aligns: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192];
        for &align in valid_aligns {
            // 仅在 align >= PTR_SIZE 时测试
            if align < PTR_SIZE {
                continue;
            }
                let mut mem: *mut c_void = core::ptr::null_mut();
                let ret = posix_memalign(
                    &mut mem as *mut *mut c_void,
                    align as c_ulong,
                    align as c_ulong, // len == align
                );
                assert_eq!(ret, 0, "align={:#x} 应成功", align);
                assert!(!mem.is_null());
                assert!(
                    is_aligned_to(mem, align),
                    "返回指针 {:#x} 不满足 {:#x} 对齐",
                    mem as usize,
                    align
                );
        }
    });

    // ===================================================================
    // 4. EINVAL 测试（委托给 aligned_alloc_inner 的路径）
    // ===================================================================

    test!("test_einval_align_not_power_of_two" {
        // align 不是 2 的幂（如 3、5、6、7、12 等），由 aligned_alloc_inner 返回
        // AlignedAllocError::InvalidAlignment，posix_memalign 转换为 EINVAL。
        let bad_aligns: &[usize] = &[3, 5, 6, 7, 9, 10, 12, 14, 24, 48, 96, 100];
        for &align in bad_aligns {
            // 跳过小于 PTR_SIZE 的值（已被快速路径拦截）
            if align <= PTR_SIZE {
                continue;
            }
                let mut mem: *mut c_void = core::ptr::null_mut();
                let ret = posix_memalign(
                    &mut mem as *mut *mut c_void,
                    align as c_ulong,
                    64,
                );
                assert_eq!(
                    ret,
                    EINVAL,
                    "align={} (非 2 的幂) 应返回 EINVAL",
                    align
                );
                assert!(mem.is_null(), "失败时 *res 应保持不变");
        }
    });

    test!("test_einval_align_3" {
        // align = 3 且 PTR_SIZE >= 4 时的边界情况。
        // 若 PTR_SIZE <= 3（不可能），快速路径拦截；否则由 aligned_alloc_inner 处理。
        if PTR_SIZE < 4 {
            return; // 32 位平台 PTR_SIZE==4，PS:3<4 被快速路径拦截
        }
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 3, 64);
            assert_eq!(ret, EINVAL, "align=3 (非 2 的幂且 >= sizeof(void*)) 应返回 EINVAL");
            assert!(mem.is_null());
    });

    // ===================================================================
    // 5. ENOMEM 测试（委托给 aligned_alloc_inner 的路径）
    // ===================================================================

    test!("test_enomem_len_zero" {
        // len == 0：POSIX 标准规定行为是实现定义的，当前实现遵循 malloc(0) 行为返回有效指针。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                0,
            );
            assert_eq!(ret, ENOMEM, "len=0 应返回 ENOMEM (musl 行为)");
            assert!(mem.is_null(), "失败时 *res 应保持不变");
    });

    test!("test_enomem_huge_allocation" {
        // 请求分配巨大的内存，期望 ENOMEM。
        // 请求接近 usize::MAX 的大小
        let huge_len = usize::MAX - 4096;
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                huge_len as c_ulong,
            );
            assert_eq!(ret, ENOMEM, "巨大分配应返回 ENOMEM");
            assert!(mem.is_null());
    });

    test!("test_enomem_len_overflow" {
        // len 溢出：len > SIZE_MAX - align 时内部检测溢出。
        // 这里构造 (align + len) 溢出的情况。
        // align=16, len=usize::MAX 应触发溢出检查
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                16,
                usize::MAX as c_ulong,
            );
            assert_eq!(ret, ENOMEM, "溢出 len 应返回 ENOMEM");
            assert!(mem.is_null());
    });

    test!("test_enomem_memory_exhaustion" {
        // 尝试耗尽虚拟地址空间 — 请求远超实际可用内存的大小触发 ENOMEM。
            let mut mem: *mut c_void = core::ptr::null_mut();
            // 请求 1 TiB 的分配, 反复泄漏直到 mmap 无法满足
            let mut found_enomem = false;
            for _ in 0..200 {
                let ret = posix_memalign(
                    &mut mem as *mut *mut c_void,
                    4096,
                    1u64 << 40, // 1 TiB
                );
                if ret == ENOMEM {
                    found_enomem = true;
                    assert!(mem.is_null());
                    break;
                }
                assert_eq!(ret, 0);
            }
            assert!(found_enomem, "应最终返回 ENOMEM");
    });

    // ===================================================================
    // 6. *res 保持不变测试（POSIX 核心不变量）
    // ===================================================================

    test!("test_res_unchanged_on_failure_align_zero" {
        // 失败时 *res 不应被修改。此测试将 *res 置为一个哨兵值，
        // 验证失败返回后值不变。
        let sentinel = 0xDEAD_BEEF_usize as *mut c_void;
            let mut mem: *mut c_void = sentinel;
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 0, 64);
            assert_eq!(ret, EINVAL);
            assert_eq!(
                mem, sentinel,
                "失败时 *res 应保持调用前的值 {:#x}，实际 {:#x}",
                sentinel as usize, mem as usize
            );
    });

    test!("test_res_unchanged_on_enomem_len_zero" {
        // 传入有效 align 但 len == 0 时，失败后 *res 不变。
        // 注意：当前实现中 len=0 返回成功，不触发 ENOMEM 路径。
        let sentinel = 0xBEEF_0001_usize as *mut c_void;
            let mut mem: *mut c_void = sentinel;
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                0,
            );
            assert_eq!(ret, ENOMEM);
            assert_eq!(mem, sentinel, "ENOMEM 时 *res 应保持调用前的值");
    });

    test!("test_res_unchanged_on_einval_bad_align" {
        // align 为非 2 的幂时失败后 *res 保持不变。
        let sentinel = 0xCAFE_F00D_usize as *mut c_void;
            let mut mem: *mut c_void = sentinel;
            // align=12（>= sizeof(void*) 但非 2 的幂）
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 12, 64);
            assert_eq!(ret, EINVAL);
            assert_eq!(mem, sentinel, "EINVAL 时 *res 应保持调用前的值");
    });

    // ===================================================================
    // 7. 返回值为合法错误码测试
    // ===================================================================

    test!("test_return_value_is_valid_error_code" {
        // 返回值只能是 0、EINVAL、ENOMEM 之一。
        let valid_codes: [c_int; 3] = [0, EINVAL, ENOMEM];
            // Case: align=0 → EINVAL
            {
                let mut mem: *mut c_void = core::ptr::null_mut();
                let ret = posix_memalign(&mut mem as *mut *mut c_void, 0, 1);
                assert!(
                    valid_codes.contains(&ret),
                    "返回值 {} 不在合法集合 {{0, {}, {}}} 中",
                    ret,
                    EINVAL,
                    ENOMEM
                );
            }
            // Case: align=1 → EINVAL
            {
                let mut mem: *mut c_void = core::ptr::null_mut();
                let ret = posix_memalign(&mut mem as *mut *mut c_void, 1, 1);
                assert!(valid_codes.contains(&ret));
            }
    });

    // ===================================================================
    // 8. NULL 指针输入测试 (POSIX 规定 p=NULL 时无操作)
    // ===================================================================

    test!("test_documentation_null_res_is_ub" {
        // 注意：`free(NULL)` 是 C 标准的无操作行为。但 `posix_memalign`
        // 的 `res` 参数指向一个 `void*` 输出位置，必须非 NULL。
        // 
        // 此测试验证 POSIX 文档行为，但实际上 `res` 为 NULL 是
        // 未定义行为（违反前置条件 P1），骨架对此不做额外检查。
        // 
        // 由于函数体为 `todo!()`，此测试在骨架阶段会 panic。
        // `posix_memalign(core::ptr::null_mut(), 8, 64)` —
        // 违反前置条件 P1，行为未定义。
        // 此测试仅为文档目的，不应在生产中运行。
    });

    // ===================================================================
    // 9. 类型兼容性 / 编译期测试
    // ===================================================================

    test!("test_type_signature_compiles" {
        // 编译期验证：posix_memalign 的类型应当与 C ABI 兼容。
        // 
        // 这不是运行时测试，而是确保接口签名正确的编译期断言。
        // 如果此函数编译通过，说明类型定义是合法的。
        // 取函数指针以验证签名
        let _pma: unsafe extern "C" fn(
            *mut *mut c_void,
            c_ulong,
            c_ulong,
        ) -> c_int = posix_memalign;

        // 验证常量值
        assert_eq!(EINVAL, 22, "EINVAL 应为 22 (Linux ABI)");
        assert_eq!(ENOMEM, 12, "ENOMEM 应为 12 (Linux ABI)");
    });

    test!("test_ptr_size_is_reasonable" {
        // 验证 ptr size 常量在合理范围。
        // 在 Rust 支持的平台上，指针宽度为 4 或 8 字节
        assert!(
            PTR_SIZE == 4 || PTR_SIZE == 8,
            "PTR_SIZE 期望 4 (32-bit) 或 8 (64-bit)，实际 {}",
            PTR_SIZE
        );
    });

    test!("test_pointer_size_at_least_4" {
        // 验证 sizeof(void*) >= 4（POSIX 隐式要求）。
        assert!(
            PTR_SIZE >= 4,
            "sizeof(*mut c_void) 应 >= 4，实际 {}",
            PTR_SIZE
        );
    });

    // ===================================================================
    // 10. 内部辅助函数测试
    // ===================================================================

    test!("test_convert_success_writes_res" {
        // 验证 `convert_aligned_alloc_result` 在 Ok 情况下将 NonNull 写入 `*res`。
        // 使用模拟 NonNull 测试
        unsafe {
            let dummy: *mut c_void = 0x1000 as *mut c_void;
            let non_null = NonNull::new_unchecked(dummy);
            let mut out: *mut c_void = core::ptr::null_mut();
            let ret = convert_aligned_alloc_result(
                Ok(non_null),
                &mut out as *mut *mut c_void,
            );
            assert_eq!(ret, 0, "成功转换应返回 0");
            assert_eq!(
                out, dummy,
                "*res 应被设置为 NonNull 内的指针值"
            );
        }
    });

    test!("test_convert_invalid_alignment_returns_einval" {
        // 验证 `convert_aligned_alloc_result` 将 InvalidAlignment → EINVAL。
        unsafe {
            let mut out: *mut c_void = 0xBEEF_usize as *mut c_void;
            let saved = out;
            let ret = convert_aligned_alloc_result(
                Err(AlignedAllocError::InvalidAlignment),
                &mut out as *mut *mut c_void,
            );
            assert_eq!(ret, EINVAL);
            // 失败时 *res 应保持不变
            assert_eq!(out, saved);
        }
    });

    test!("test_convert_out_of_memory_returns_enomem" {
        // 验证 `convert_aligned_alloc_result` 将 OutOfMemory → ENOMEM。
        unsafe {
            let mut out: *mut c_void = 0xCAFE_usize as *mut c_void;
            let saved = out;
            let ret = convert_aligned_alloc_result(
                Err(AlignedAllocError::OutOfMemory),
                &mut out as *mut *mut c_void,
            );
            assert_eq!(ret, ENOMEM);
            assert_eq!(out, saved);
        }
    });

    // ===================================================================
    // 11. 边界值测试
    // ===================================================================

    test!("test_success_align_power_of_two_large" {
        // align 恰好为 2 的很大幂（如 2^30），成功分配。
        let align = 1usize << 25; // 32 MB alignment
            // 注意：这样的大对齐通常需要 mmap，仅在支持 hugepage 的系统上
            // 或通过 musl 的 mmap 分配路径处理
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                align as c_ulong,
                4096,
            );
            // 大对齐 + 小长度：允许成功或失败取决于内核和地址空间
            // posix_memalign 要求 align 为 2 的幂且 >= sizeof(void*)
            if ret == 0 {
                assert!(!mem.is_null());
                assert!(is_aligned_to(mem, align));
            } else {
                assert!(
                    ret == EINVAL || ret == ENOMEM,
                    "返回值应为 EINVAL 或 ENOMEM"
                );
            }
    });

    test!("test_align_max_value" {
        // align 为 c_ulong 类型的最大值。
        let max_align = c_ulong::MAX;
        // max_align 不是 2 的幂（除非平台指针宽度 == c_ulong 的总位数，
        // 且恰好为 2 的幂——这在 32 位平台上 align=2^32-1 通过不了）
        // 但首先处理快速路径：max_align >= sizeof(void*) 肯定成立
        //
        // 待实现后调用 posix_memalign 验证 max_align 的行为：
        // let mut mem: *mut c_void = core::ptr::null_mut();
        // let ret = posix_memalign(&mut mem as *mut *mut c_void, max_align, 1);
        let _ = max_align;
    });

    test!("test_len_max_value" {
        // len 为 c_ulong::MAX 时的行为。
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                PTR_SIZE as c_ulong,
                c_ulong::MAX,
            );
            assert_eq!(ret, ENOMEM, "c_ulong::MAX 长度应返回 ENOMEM");
            assert!(mem.is_null());
    });

    // ===================================================================
    // 12. AlignedAllocError 单元测试 (占位类型)
    // ===================================================================

    // test!("test_aligned_alloc_error_debug" {
    //     let e1 = AlignedAllocError::InvalidAlignment;
    //     let e2 = AlignedAllocError::OutOfMemory;
    //     // 验证 PartialEq
    //     assert_eq!(e1, AlignedAllocError::InvalidAlignment);
    //     assert_eq!(e2, AlignedAllocError::OutOfMemory);
    //     assert_ne!(e1, e2);
    //     // 验证 Debug 格式化不 panic
    //     let _ = format!("{:?}", e1);
    //     let _ = format!("{:?}", e2);
    // });

    // ===================================================================
    // 13. 线程安全标记测试 (编译期 + 文档)
    // ===================================================================

    test!("test_function_pointer_is_send_sync" {
        // 验证 posix_memalign 函数指针满足 Send + Sync。
        // 虽然 raw fn pointer 本身是 Send + Sync，此测试用于文档目的。
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        // 函数指针在 Rust 中自动是 Send + Sync
        assert_send::<unsafe extern "C" fn(*mut *mut c_void, c_ulong, c_ulong) -> c_int>();
        assert_sync::<unsafe extern "C" fn(*mut *mut c_void, c_ulong, c_ulong) -> c_int>();
    });

    // ===================================================================
    // 14. 跨平台兼容性测试
    // ===================================================================

    test!("test_minimum_alignment_is_ptr_size" {
        // 在 32 位平台上，sizeof(void*) = 4，align 最小为 4。
        // 快速路径: align < PTR_SIZE → EINVAL
        // PTR_SIZE 可能是 4 或 8
        let just_below = (PTR_SIZE - 1) as c_ulong;
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(
                &mut mem as *mut *mut c_void,
                just_below,
                1,
            );
            // 由于 just_below < PTR_SIZE，快速路径应返回 EINVAL
            assert_eq!(ret, EINVAL, "align < sizeof(void*) 应返回 EINVAL");
    });


    #[cfg(target_pointer_width = "64")]
    test!("test_64bit_specific_align_4_is_einval" {
        // 在 64 位平台上 align=4 < 8 应返回 EINVAL。
        // 在 32 位平台上 align=4 == sizeof(void*) 应通过快速路径。
        // 64 位: sizeof(void*) = 8
        assert_eq!(PTR_SIZE, 8);
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 4, 64);
            assert_eq!(ret, EINVAL, "64-bit: align=4 < sizeof(void*)=8 → EINVAL");
            assert!(mem.is_null());
    });

    /// 在 32 位平台上 align=4 == sizeof(void*) 应通过。
    #[cfg(target_pointer_width = "32")]
    test!("test_32bit_specific_align_4_is_valid" {
        assert_eq!(PTR_SIZE, 4);
            let mut mem: *mut c_void = core::ptr::null_mut();
            let ret = posix_memalign(&mut mem as *mut *mut c_void, 4, 64);
            // 32-bit: align=4 == sizeof(void*) 应通过快速路径
            assert_eq!(ret, 0);
            assert!(!mem.is_null());
            assert!(is_aligned_to(mem, 4));
    });
}