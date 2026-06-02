//! 原始系统调用封装 — lite_malloc 模块的 syscall 层。
//!
//! 提供:
//! - `sys_brk`: brk 系统调用封装
//! - `sys_mmap`: mmap 系统调用封装
//!
//! 所有 syscall 通过 `core::arch::asm!` 内联汇编直接发起，不使用 `libc` crate。

use super::*;
use core::sync::atomic::Ordering;

/// 发起 brk 系统调用，扩展/获取数据段末尾地址。
///
/// # 参数
/// - `addr == 0`: 返回当前 brk 值（获取语义）
/// - `addr != 0`: 尝试设置 brk 为 `addr`，返回新的 brk 值（失败时返回旧值）
///
/// # 返回值
/// - 当前 brk 值（对于 `addr == 0`）或设置后的新 brk 值
///
/// # 安全性
///
/// 调用者必须确保:
/// - 内核已完成初始化，系统调用机制可用
/// - `addr` 是有效/合理的地址（内核会校验但不是所有内核版本都会妥善处理不合理的值）
#[inline]
pub(crate) unsafe fn sys_brk(addr: usize) -> usize {
    rusl_internal::do_syscall!(SYS_BRK as i64, addr) as usize
}

/// 发起 mmap 系统调用，创建内存映射。
///
/// 封装 Linux mmap 系统调用，支持匿名映射用于动态内存分配。
///
/// # 参数
/// - `addr`: 建议的映射起始地址（传 `null_mut()` 由内核选择）
/// - `len`: 映射长度（字节，必须 > 0）
/// - `prot`: 内存保护标志（`PROT_READ | PROT_WRITE` 等）
/// - `flags`: 映射标志（`MAP_PRIVATE | MAP_ANONYMOUS` 等）
/// - `fd`: 文件描述符（匿名映射传 -1）
/// - `offset`: 文件偏移（匿名映射传 0）
///
/// # 返回值
/// - **成功**: 返回映射区域的起始地址（内核选择的虚拟地址）
/// - **失败**: 返回 `MAP_FAILED`（即 `!0usize`），`errno` 由内核设置
///
/// # 安全性
///
/// 调用者必须确保:
/// - 内核已完成初始化
/// - `len > 0`
/// - `prot` / `flags` 为有效的标志组合
#[inline]
pub(crate) unsafe fn sys_mmap(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: isize,
) -> *mut c_void {
    rusl_internal::do_syscall!(SYS_MMAP as i64, addr, len, prot, flags, fd, offset) as *mut c_void
}

/// 发起 munmap 系统调用，解除内存映射。
#[inline]
pub(crate) unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> c_int {
    rusl_internal::do_syscall!(SYS_MUNMAP as i64, addr, len) as c_int
}

// ===========================================================================
// 单元测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    // ---- 编译期验证 ----

    test!("test_sys_brk_signature" {
        // 验证 sys_brk 函数签名存在且可编译。
        // 仅验证编译，不实际调用（因为 syscall 在测试环境可能不安全）
        let _f: unsafe fn(usize) -> usize = sys_brk;
    });

    test!("test_sys_mmap_signature" {
        // 验证 sys_mmap 函数签名存在且可编译。
        let _f: unsafe fn(*mut c_void, usize, c_int, c_int, c_int, isize) -> *mut c_void = sys_mmap;
    });

    // ---- sys_brk 语义测试 ----

    test!("test_sys_brk_get_current" {
        // 验证: `sys_brk(0)` 应返回当前 brk 值（不改变状态）。
        // 注: 在 todo!() 骨架中此测试预期 panic，实际测试逻辑为示例。
        unsafe {
            let brk_before = sys_brk(0);
            let brk_after = sys_brk(0);
            // 连续两次 brk(0) 应返回相同值（未修改状态）
            assert_eq!(brk_before, brk_after);
            assert_ne!(brk_before, 0);
        }
    });

    test!("test_sys_brk_page_aligned" {
        // 验证: `sys_brk` 返回值为页对齐。
        unsafe {
            let brk = sys_brk(0);
            let page_size = PAGE_SIZE.load(Ordering::Relaxed) as usize;
            assert_eq!(brk % page_size, 0, "brk 值 {} 应为页对齐", brk);
        }
    });

    // ---- sys_mmap 语义测试 ----

    test!("test_sys_mmap_single_page_success" {
        // 验证: `sys_mmap` 请求单页匿名映射成功，返回非 MAP_FAILED 指针。
        unsafe {
            let page_size = PAGE_SIZE.load(Ordering::Relaxed) as usize;
            let ptr = sys_mmap(
                core::ptr::null_mut(),
                page_size,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(
                ptr as usize != MAP_FAILED,
                "匿名 mmap 应成功"
            );
            // 返回值应为页对齐
            assert_eq!(ptr as usize % page_size, 0);
            // 清理
            sys_munmap(ptr, page_size);
        }
    });

    test!("test_sys_mmap_zero_len_fails" {
        // 验证: `sys_mmap(len=0)` 应失败（无效参数）。
        unsafe {
            let ptr = sys_mmap(
                core::ptr::null_mut(),
                0,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert_eq!(ptr as usize, MAP_FAILED);
        }
    });

    test!("test_sys_mmap_multiple_pages" {
        // 验证: `sys_mmap` 多页映射返回连续可用区域。
        unsafe {
            let page_size = PAGE_SIZE.load(Ordering::Relaxed) as usize;
            let len = page_size * 4;
            let ptr = sys_mmap(
                core::ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(ptr as usize != MAP_FAILED);
            assert_eq!(ptr as usize % page_size, 0);
            // 清理
            sys_munmap(ptr, len);
        }
    });

    // ---- 类型正确性测试 ----

    test!("test_sys_mmap_return_type" {
        // 验证 sys_mmap 返回类型可用于后续指针操作。
        // 返回值类型应与 *mut c_void 兼容
        let _f: fn(*mut c_void, usize, c_int, c_int, c_int, isize) -> *mut c_void =
            |addr, len, prot, flags, fd, offset| unsafe {
                sys_mmap(addr, len, prot, flags, fd, offset)
            };
    });

    // ---- MAP_FAILED 边界测试 ----

    test!("test_map_failed_bit_pattern" {
        // 验证 MAP_FAILED 的位模式为全 1（usize::MAX）。
        assert_eq!(MAP_FAILED, !0usize);
        assert_eq!(MAP_FAILED.wrapping_add(1), 0);
    });
}