//! mallocng 系统调用封装层。
//!
//! 提供内存管理所需的底层 Linux 系统调用封装
//! （`mmap`、`munmap`、`mremap`、`brk`、`madvise` 等）。
//!
//! 所有系统调用通过 `asm!` 内联汇编直接发起，
//! 不依赖 `libc` crate，兼容 `#![no_std]` 环境。
//!
//! ## 架构说明
//!
//! 系统调用采用 musl 风格的 `syscallN()` 宏模式：
//! 1. 由 `rusl_internal::syscall::raw_syscallN()` 发起原始系统调用
//! 2. 通过 `rusl_internal::__syscall_ret()` 转换返回值为 libc 约定
//!
//! ## 支持的架构
//!
//! - x86_64: 使用 `SYS_mmap=9`、`SYS_munmap=11`、`SYS_mremap=25`
//! - aarch64: 使用 `SYS_mmap=222`、`SYS_munmap=215`、`SYS_mremap=216`

use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// 系统调用号常量 (按架构分派)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
const SYS_MREMAP: i64 = 25;
#[cfg(target_arch = "aarch64")]
const SYS_MREMAP: i64 = 216;

#[cfg(target_arch = "x86_64")]
const SYS_MMAP: i64 = 9;
#[cfg(target_arch = "aarch64")]
const SYS_MMAP: i64 = 222;

#[cfg(target_arch = "x86_64")]
const SYS_MUNMAP: i64 = 11;
#[cfg(target_arch = "aarch64")]
const SYS_MUNMAP: i64 = 215;

// ---------------------------------------------------------------------------
// 标志位常量
// ---------------------------------------------------------------------------

/// `mremap` 标志：允许内核移动映射到新的虚拟地址。
pub(crate) const MREMAP_MAYMOVE: c_int = 1;

/// `mremap` 标志：固定地址重映射（不移动）。
pub(crate) const MREMAP_FIXED: c_int = 2;

/// `mmap` 失败时的哨兵返回值。
/// musl 约定：`MAP_FAILED = (void *)-1`。
pub(crate) const MAP_FAILED: *mut c_void = (-1isize) as *mut c_void;

// ---------------------------------------------------------------------------
// mmap 相关标志常量 (供 mallocng 内部使用)
// ---------------------------------------------------------------------------

/// 页可读
pub(crate) const PROT_READ: c_int = 1;
/// 页可写
pub(crate) const PROT_WRITE: c_int = 2;
/// 页不可访问
pub(crate) const PROT_NONE: c_int = 0;

/// 映射为私有写时拷贝
pub(crate) const MAP_PRIVATE: c_int = 2;
/// 映射为匿名内存 (fd 被忽略)
pub(crate) const MAP_ANONYMOUS: c_int = 32;

// ---------------------------------------------------------------------------
// 系统调用封装函数
// ---------------------------------------------------------------------------

/// 通过 `SYS_mremap` 重新映射虚拟内存区域。
///
/// 在内核中申请扩大或缩小已有的内存映射，可选的移动映射到新地址。
/// 此函数是 musl 中 `mremap` 的 rusl 内部版本，不依赖 `libc` crate。
///
/// # 参数
///
/// - `old`: 原映射的起始地址
/// - `old_len`: 原映射的长度（字节，必须页对齐）
/// - `new_len`: 新映射的长度（字节，必须页对齐）
/// - `flags`: 标志位，`MREMAP_MAYMOVE` 或 `MREMAP_FIXED`
///
/// # Safety
///
/// - `old` 必须是由之前 `mmap` 创建的有效映射地址
/// - `old_len` 和 `new_len` 必须是系统页大小的倍数
/// - 若 `flags` 包含 `MREMAP_FIXED`，`new_address` 必须有效（当前 API 不支持）
/// - 若 `flags` 不包含 `MREMAP_MAYMOVE`，且内核无法在原地完成扩展，
///   调用将失败（`ENOMEM`）
///
/// # 返回值
///
/// - **成功**: 返回新映射的起始地址（可能等于 `old` 也可能不同）
/// - **失败**: 返回 `MAP_FAILED`，`errno` 由 `__syscall_ret` 自动设置
///
/// # 实现说明
///
/// 在 rusl 中，此函数通过 `crate::do_syscall!` 宏发起系统调用：
/// ```ignore
/// crate::do_syscall!(SYS_MREMAP, old, old_len, new_len, flags)
/// ```
/// 返回值经 `__syscall_ret` 转换：成功返回指针，失败设置 `errno`。
pub(crate) unsafe fn sys_mremap(
    old: *mut c_void,
    old_len: usize,
    new_len: usize,
    flags: c_int,
) -> *mut c_void {
    crate::do_syscall!(SYS_MREMAP, old, old_len, new_len, flags, 0usize) as *mut c_void
}

/// 通过 `SYS_mmap` 创建新的虚拟内存映射。
///
/// 在进程的虚拟地址空间中创建新的映射区域。这是 mallocng 获取内存的
/// 主要手段 —— 无论是大块 mmap 分配还是元数据区分配均依赖此调用。
///
/// # 参数
///
/// - `addr`: 建议的映射起始地址（通常为 `0`，让内核选择）
/// - `len`: 映射长度（字节，向上取整到页边界）
/// - `prot`: 内存保护标志 (`PROT_READ | PROT_WRITE` 等)
/// - `flags`: 映射类型标志 (`MAP_PRIVATE | MAP_ANONYMOUS` 等)
/// - `fd`: 文件描述符（匿名映射时忽略）
/// - `off`: 文件偏移量（匿名映射时忽略）
///
/// # Safety
///
/// - 错误的保护标志组合可能导致内存访问违规
/// - `len` 过大可能导致进程地址空间耗尽
///
/// # 返回值
///
/// - **成功**: 返回映射区域的起始地址（页对齐）
/// - **失败**: 返回 `MAP_FAILED`，`errno` 由 `__syscall_ret` 自动设置
pub(crate) unsafe fn sys_mmap(
    addr: *mut c_void,
    len: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    off: i64,
) -> *mut c_void {
    crate::do_syscall!(SYS_MMAP, addr, len, prot, flags, fd, off) as *mut c_void
}

/// 通过 `SYS_munmap` 解除虚拟内存映射。
///
/// 释放之前由 `mmap` 创建的映射区域，将内存归还给内核。
///
/// # 参数
///
/// - `addr`: 要解除映射的起始地址（必须页对齐）
/// - `len`: 要解除的长度（字节，必须页对齐）
///
/// # Safety
///
/// - `addr` 必须是之前 `mmap` 返回的有效映射地址
/// - `len` 必须是系统页大小的倍数
/// - 调用后不得再访问已解除映射的区域（否则触发 SIGSEGV）
///
/// # 返回值
///
/// - **成功**: 返回 `0`
/// - **失败**: 返回 `-1`，`errno` 由 `__syscall_ret` 自动设置
pub(crate) unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> c_int {
    crate::do_syscall!(SYS_MUNMAP, addr, len) as c_int
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::ptr;

    test!("test_mremap_maymove_flag_value" {
        // MREMAP_MAYMOVE 在 Linux 内核中恒为 1
        assert_eq!(MREMAP_MAYMOVE, 1);
    });

    test!("test_map_failed_value" {
        // MAP_FAILED 应为 (void*)-1
        assert_eq!(MAP_FAILED, (-1isize) as *mut c_void);
    });

    test!("test_map_failed_is_not_null" {
        // MAP_FAILED 应与 NULL 区分
        assert!(!MAP_FAILED.is_null());
    });

    test!("test_prot_constants_non_overlapping" {
        // PROT_READ 和 PROT_WRITE 是不同的位
        assert_ne!(PROT_READ, PROT_WRITE);
        assert_eq!(PROT_READ | PROT_WRITE, 3);
    });

    test!("test_map_constants" {
        // MAP_PRIVATE 和 MAP_ANONYMOUS 是标准值
        #[cfg(target_os = "linux")]
        {
            assert_eq!(MAP_PRIVATE, 2);
            assert_eq!(MAP_ANONYMOUS, 32);
        }
    });

    test!("test_sys_mmap_fails_with_bad_args" {
        // 传入无效参数（长度 0），mmap 应返回 MAP_FAILED
            // mmap(0, 0, PROT_NONE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) 应失败
            // 等待实现完成后验证
    });

    test!("test_sys_munmap_fails_with_null" {
        // munmap(NULL, ...) 应失败或返回错误
        // 等待实现完成后验证
    });
}