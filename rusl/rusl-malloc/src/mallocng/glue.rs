// glue.rs — 分配器基础设施胶水层
//
// 对应 musl 的 src/malloc/mallocng/glue.h
// 本模块是 rusl mallocng 分配器与 rusl 内部基础设施之间的胶水适配层。
//
// 核心职责:
// 1. 封装系统调用接口（brk/mmap/madvise/mremap/munmap/mprotect）
// 2. 提供统一的锁原语（基于 futex 的自旋锁，支持 atfork）
// 3. 提供线程安全检测、随机密钥生成等辅助基础设施
//
// 所有符号为 pub(crate)，仅 mallocng 子系统内部可见。
// 唯一的例外是 __malloc_atfork，通过 #[cfg_attr(not(any(test, feature = "c-test")), no_mangle)] 导出为 C ABI 符号。

use core::ffi::{c_int, c_void};
use core::sync::atomic::{AtomicI32, Ordering};

use super::dynlink::{__aligned_alloc_replaced, __malloc_replaced};
// use crate::import::{__syscall1, __syscall4, __syscall3};

// ============================================================================
// 系统调用号常量 (按架构分派)
// ============================================================================

/// SYS_madvise — 内存建议系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_MADVISE: i64 = 28;
#[cfg(target_arch = "aarch64")]
const SYS_MADVISE: i64 = 233;

/// SYS_mprotect — 内存保护系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_MPROTECT: i64 = 10;
#[cfg(target_arch = "aarch64")]
const SYS_MPROTECT: i64 = 226;

/// SYS_futex — 快速用户空间锁系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_FUTEX: i64 = 202;
#[cfg(target_arch = "aarch64")]
const SYS_FUTEX: i64 = 98;

/// SYS_mremap — 内存重映射系统调用号
/// 注: 与 super::syscall 模块中的 SYS_MREMAP 值一致
#[cfg(target_arch = "x86_64")]
const SYS_MREMAP: i64 = 25;
#[cfg(target_arch = "aarch64")]
const SYS_MREMAP: i64 = 216;

/// SYS_brk — 程序堆顶系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_BRK: i64 = 12;
#[cfg(target_arch = "aarch64")]
const SYS_BRK: i64 = 214;

/// SYS_mmap — 内存映射系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_MMAP: i64 = 9;
#[cfg(target_arch = "aarch64")]
const SYS_MMAP: i64 = 222;

/// SYS_munmap — 解除内存映射系统调用号
#[cfg(target_arch = "x86_64")]
const SYS_MUNMAP: i64 = 11;
#[cfg(target_arch = "aarch64")]
const SYS_MUNMAP: i64 = 215;

// ============================================================================
// futex 与锁内部常量
// ============================================================================

/// futex 等待操作码
const FUTEX_WAIT: i32 = 0;
/// futex 唤醒操作码
const FUTEX_WAKE: i32 = 1;
/// futex 私有标志（进程内优化，避免 VMA 查找）
const FUTEX_PRIVATE: i32 = 128;

/// INT_MIN 作为自旋锁的"已锁定"标志位 (musl __lock 算法核心)
///
/// musl 的 __lock 使用 INT_MIN 作为锁标志位，并用低位寄存器(concession count)追踪竞争:
/// - `0`: 未锁定，无线程在临界区
/// - `< 0`: 已锁定，同辰量为 `x - INT_MIN`（含锁持有者）
/// - `> 0`: 未锁定但有同辰线程（仅理论可能）
/// - `INT_MIN + 1` (= -2147483647): 已锁定且恰好 1 个持有者（fast-path 目标状态）
const INT_MIN: i32 = i32::MIN;

// ============================================================================
// 系统调用封装函数
// ============================================================================

/// 扩展进程堆 (brk) 区域。
///
/// 通过 SYS_brk 系统调用设置新的 program break 地址。
///
/// # 参数
///
/// - `p`: 新的 program break 地址，传入 `0` 表示查询当前 brk 值
///
/// # 返回值
///
/// - 成功时返回新的 program break 地址
/// - 失败时返回旧 break 值（不等于 `p`），由调用者 `alloc_meta()` 处理
///
/// # Safety
///
/// - `p` 必须是有效的内存地址或 0（查询当前 brk）
/// - 调用者负责处理返回值以判断成功/失败
///
/// # 调用上下文
///
/// 仅在 `alloc_meta()` 中被调用，用于扩展 meta_area 页面。
pub(crate) unsafe fn brk(p: usize) -> usize {
    do_syscall!(SYS_BRK, p as i64) as usize
}

/// 内存映射系统调用封装。
///
/// 通过 SYS_mmap 系统调用在进程虚拟地址空间中创建新的映射区域。
/// 此函数不依赖 libc crate，直接通过内联汇编发起系统调用。
///
/// # 参数
///
/// - `addr`: 建议的映射起始地址（通常为空指针，由内核选择）
/// - `length`: 映射长度（字节）
/// - `prot`: 内存保护标志（PROT_READ、PROT_WRITE、PROT_NONE 等组合）
/// - `flags`: 映射类型标志（MAP_PRIVATE、MAP_ANONYMOUS 等组合）
/// - `fd`: 文件描述符（匿名映射时忽略）
/// - `offset`: 文件偏移量（匿名映射时忽略，单位字节）
///
/// # 返回值
///
/// - 成功时返回映射区域的起始地址
/// - 失败时返回 `null_mut()` (MAP_FAILED)，errno 由系统调用层设置
///
/// # Safety
///
/// - 参数语义与 POSIX `mmap` 一致
/// - 返回的指针可能为 `core::ptr::null_mut()` 表示 MAP_FAILED
/// - 映射区域在使用完毕后必须通过 `munmap()` 释放
pub(crate) unsafe fn mmap(
    addr: *mut c_void,
    length: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: i64,
) -> *mut c_void {
    crate::do_syscall!(SYS_MMAP, addr, length, prot, flags, fd, offset) as *mut c_void
}

/// 内存建议系统调用封装。
///
/// 通过 SYS_madvise 系统调用向内核提供内存使用建议。
/// 在 mallocng 中用于 `free()` 路径向内核提示可回收页面。
///
/// # 参数
///
/// - `addr`: 起始地址（必须页对齐）
/// - `length`: 长度（字节，必须 > 0）
/// - `advice`: 建议类型（MADV_FREE、MADV_DONTNEED 等）
///
/// # 返回值
///
/// - 成功时返回 0
/// - 失败时返回 -1（errno 由系统调用层设置）
///
/// # Safety
///
/// - `addr` 必须页对齐
/// - `length` 必须 > 0
/// - 参数语义与 POSIX `madvise` 一致
pub(crate) unsafe fn madvise(addr: *mut c_void, length: usize, advice: c_int) -> c_int {
    crate::do_syscall!(SYS_MADVISE, addr, length, advice) as c_int
}

/// 内存重映射系统调用封装（Linux 特定）。
///
/// 通过 SYS_mremap 系统调用在内核中调整已有内存映射的大小。
/// 可选地允许内核将映射移动到新地址。
/// 在 mallocng 中用于 `realloc` 的大块映射原地/迁移扩展优化。
///
/// # 参数
///
/// - `old_addr`: 原映射的起始地址
/// - `old_len`: 原映射的长度（字节）
/// - `new_len`: 新映射的长度（字节）
/// - `flags`: 标志位（MREMAP_MAYMOVE 等）
/// - `new_addr`: 建议的新地址（MREMAP_FIXED 时使用，通常为 null）
///
/// # 返回值
///
/// - 成功时返回新映射的起始地址（可能与 `old_addr` 相同或不同）
/// - 失败时返回 `null_mut()` (MAP_FAILED)，errno 由系统调用层设置
///
/// # Safety
///
/// - `old_addr` 必须是之前由 `mmap` 创建的有效映射地址
/// - `old_len` 和 `new_len` 必须是系统页大小的倍数
/// - 参数语义与 Linux `mremap` 一致
pub(crate) unsafe fn mremap(
    old_addr: *mut c_void,
    old_len: usize,
    new_len: usize,
    flags: c_int,
    new_addr: *mut c_void,
) -> *mut c_void {
    crate::do_syscall!(SYS_MREMAP, old_addr, old_len, new_len, flags, new_addr) as *mut c_void
}

/// 解除内存映射系统调用封装。
///
/// 通过 SYS_munmap 系统调用释放之前由 `mmap` 创建的映射区域。
///
/// # 参数
///
/// - `addr`: 要解除映射的起始地址（必须页对齐）
/// - `length`: 要解除的长度（字节，必须 > 0）
///
/// # 返回值
///
/// - 成功时返回 0
/// - 失败时返回 -1（errno 由系统调用层设置）
///
/// # Safety
///
/// - `addr` 必须是之前 `mmap` 返回的有效映射地址
/// - `length` 必须是系统页大小的倍数
/// - 调用后不得再访问已解除映射的区域（否则触发 SIGSEGV）
/// - 参数语义与 POSIX `munmap` 一致
pub(crate) unsafe fn munmap(addr: *mut c_void, length: usize) -> c_int {
    crate::do_syscall!(SYS_MUNMAP, addr, length) as c_int
}

/// 内存保护系统调用封装。
///
/// 通过 SYS_mprotect 系统调用修改内存区域的访问保护属性。
/// 在 mallocng 中用于设置 meta_area 的保护页（PROT_NONE），
/// 防止堆溢出破坏元数据。
///
/// # 参数
///
/// - `addr`: 起始地址（必须页对齐）
/// - `length`: 长度（字节）
/// - `prot`: 新的保护标志（PROT_READ、PROT_WRITE、PROT_NONE 等组合）
///
/// # 返回值
///
/// - 成功时返回 0
/// - 失败时返回 -1（errno 由系统调用层设置）
///
/// # Safety
///
/// - `addr` 必须页对齐
/// - 设置 PROT_NONE 后访问该区域将触发 SIGSEGV（这是预期行为，用于守卫页）
/// - 参数语义与 POSIX `mprotect` 一致
pub(crate) unsafe fn mprotect(addr: *mut c_void, length: usize, prot: c_int) -> c_int {
    crate::do_syscall!(SYS_MPROTECT, addr, length, prot) as c_int
}

// ============================================================================
// 内存建议 (madvise) 常量
// ============================================================================

/// Linux madvise 参数: 告知内核可惰性回收指定地址范围的物理页面。
///
/// Linux 4.5+ 特性。标记后，内核在内存压力下才实际回收，
/// 在此之前进程可继续访问（内容保持有效）。性能优于 MADV_DONTNEED，
/// 但 RSS 统计不立即更新。
pub(crate) const MADV_FREE: c_int = 8;

/// Linux madvise 参数: 告知内核不再需要指定地址范围的物理页面，立即回收。
///
/// 标记后再次访问将触发缺页，页面内容清零。RSS 立即下降。
/// 适用于确定不再使用的内存区域。
pub(crate) const MADV_DONTNEED: c_int = 4;

/// Linux madvise 参数: 告知内核指定地址范围将很快被访问（预取提示）。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_WILLNEED: c_int = 3;

/// Linux madvise 参数: 告知内核按正常模式访问（默认行为，取消此前建议）。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_NORMAL: c_int = 0;

/// Linux madvise 参数: 告知内核将按随机访问模式访问。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_RANDOM: c_int = 1;

/// Linux madvise 参数: 告知内核将按顺序访问模式访问。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_SEQUENTIAL: c_int = 2;

/// Linux madvise 参数: 告知内核合并 (merge) 指定范围内的 KSM (Kernel Same-page Merging) 页面。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_MERGEABLE: c_int = 12;

/// Linux madvise 参数: 告知内核取消合并 (unmerge) 指定范围内的 KSM 页面。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_UNMERGEABLE: c_int = 13;

/// Linux madvise 参数: 告知内核该区域将不再被访问（类似于 MADV_DONTNEED，
/// 但在某些平台上仅影响核心转储行为）。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_DONTDUMP: c_int = 16;

/// Linux madvise 参数: 撤销 MADV_DONTDUMP 的效果。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_DODUMP: c_int = 17;

/// Linux madvise 参数: 告知内核该区域将被用于硬件错误恢复 (Linux 4.9+)。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_HWPOISON: c_int = 100;

/// Linux madvise 参数: 软离线页面 (Linux 4.9+)。
/// 当前仅作为常量定义，mallocng 未直接使用。
#[allow(dead_code)]
pub(crate) const MADV_SOFT_OFFLINE: c_int = 101;

// ============================================================================
// 内存映射 (mmap/mremap/mprotect) 标志常量
// ============================================================================

/// mmap 保护标志: 页可读
pub(crate) const PROT_READ: c_int = 1;

/// mmap 保护标志: 页可写
pub(crate) const PROT_WRITE: c_int = 2;

/// mmap 保护标志: 页可执行
pub(crate) const PROT_EXEC: c_int = 4;

/// mmap 保护标志: 页不可访问（用于守卫页）
pub(crate) const PROT_NONE: c_int = 0;

/// mmap 映射标志: 映射为私有写时拷贝
pub(crate) const MAP_PRIVATE: c_int = 2;

/// mmap 映射标志: 映射为共享内存
#[allow(dead_code)]
pub(crate) const MAP_SHARED: c_int = 1;

/// mmap 映射标志: 映射为匿名内存（fd 被忽略）
pub(crate) const MAP_ANONYMOUS: c_int = 32;

/// mmap 映射标志: 固定地址映射（若地址不可用则失败）
#[allow(dead_code)]
pub(crate) const MAP_FIXED: c_int = 16;

/// mmap 失败时的哨兵返回值。
/// musl 约定: MAP_FAILED = (void *)-1
pub(crate) const MAP_FAILED: *mut c_void = (-1isize) as *mut c_void;

/// mremap 标志: 允许内核将映射移动到新的虚拟地址
#[allow(dead_code)]
pub(crate) const MREMAP_MAYMOVE: c_int = 1;

/// mremap 标志: 固定地址重映射（不移动，原地失败则报错）
#[allow(dead_code)]
pub(crate) const MREMAP_FIXED: c_int = 2;

/// mremap 标志 (Linux 5.7+): 指定的 new_addr 用于提示内核选择有利地址，
/// 但映射可能被放置在其他位置。
#[allow(dead_code)]
pub(crate) const MREMAP_DONTUNMAP: c_int = 4;

// ============================================================================
// 运行时配置常量
// ============================================================================

/// 控制 `free()` 中是否使用 `MADV_FREE` 归还物理页面。
///
/// - `false` (默认): 禁用 MADV_FREE，使用保守的 MADV_DONTNEED 策略。
///   页面立即可被内核回收，RSS 统计准确。
/// - `true`: 在 `free()` 的 madvise 路径中优先使用 MADV_FREE。
///   延迟回收，性能更优但 RSS 统计可能不精确。
///
/// 当前设为 `false`，保持与 musl 默认配置一致。
pub(crate) const USE_MADV_FREE: bool = false;

/// 锁语义配置：当为 `true` 时，读锁和写锁使用相同的互斥锁（无读写区分，都是排他锁）。
///
/// 在 mallocng 场景下，读写者并无真正的并发收益——几乎所有对全局 ctx 的修改
/// 都需要排他访问。设为 `true` 简化了锁语义和 fast-path 的实现。
///
/// 使用位置：`malloc()` 函数 fast-path，若 RDLOCK_IS_EXCLUSIVE 则直接本地更新
/// `avail_mask`，无需额外的读锁/写锁状态跟踪。
pub(crate) const RDLOCK_IS_EXCLUSIVE: bool = true;

// ============================================================================
// 页大小
// ============================================================================

/// 系统页大小常量（Linux 默认 4096 字节）。
///
/// 在 x86_64 和 aarch64 Linux 平台上，页大小几乎总是 4KB。
/// 编译时常量提供最佳优化机会。
#[cfg(target_os = "linux")]
pub(crate) const PAGE_SIZE: usize = 4096;

/// 运行时页大小获取函数（用于编译时无法确定页大小的架构）。
///
/// 从 `ctx.pagesize` 读取，该值在分配器初始化阶段由 auxv (AT_PAGESZ) 获取。
/// 在已知页大小的架构上（如 x86_64 / aarch64 Linux），应优先使用 `PAGE_SIZE` 常量。
pub(crate) fn page_size() -> usize {
    // 在已知页大小的架构上（x86_64 / aarch64 Linux），直接使用编译时常量。
    // 如果将来需要运行时页大小检测，可通过 CTX.pagesize 读取。
    PAGE_SIZE
}

// ============================================================================
// 运行时条件函数
// ============================================================================

/// 运行时条件判断：当用户替换了 `malloc` 但未替换 `aligned_alloc` 时返回 `true`。
///
/// 此时 `aligned_alloc()` 应设置 `errno = ENOMEM` 并返回 NULL，
/// 以防止在交叉替换场景下的不一致行为。
///
/// 等价于 C 侧的 `DISABLE_ALIGNED_ALLOC` 宏:
/// ```c
/// #define DISABLE_ALIGNED_ALLOC (__malloc_replaced && !__aligned_alloc_replaced)
/// ```
///
/// # 前置条件
///
/// 动态链接器已完成初始化（`__malloc_replaced` / `__aligned_alloc_replaced` 已确定）。
/// 在 rusl `no_std` 静态链接环境下，两个标志均为 false，此函数始终返回 false。
///
/// # 依赖
///
/// - `super::dynlink::__malloc_replaced`: 标记 malloc 是否被外部动态库替换
/// - `super::dynlink::__aligned_alloc_replaced`: 标记 aligned_alloc 是否被外部动态库替换
pub(crate) fn disable_aligned_alloc() -> bool {
    // 等价于 C 的宏: #define DISABLE_ALIGNED_ALLOC (__malloc_replaced && !__aligned_alloc_replaced)
    // rusl no_std 环境下两个标志均为 false，此函数始终返回 false。
    // 使用 Relaxed 排序即可：标志由动态链接器在启动时设置，之后不再变化。
    __malloc_replaced.load(Ordering::Relaxed) && !__aligned_alloc_replaced.load(Ordering::Relaxed)
}

/// 运行时检测是否需要加锁。
///
/// 当进程为单线程时（`need_locks == false`），返回 `false`，
/// 跳过所有锁操作以提升性能。
///
/// 等价于 C 的 `MT` 宏 (`libc.need_locks`)。
///
/// # 使用场景
///
/// 在 `MallocLock::rdlock()` / `wrlock()` / `unlock()` / `atfork()` 等
/// 所有锁操作路径的最外层使用，决定是否执行实际的锁操作。
///
/// # 依赖
///
/// - `crate::runtime::need_locks()`: rusl 全局运行时状态访问器
///   (TODO: 模块尚未实现，待 `src/runtime.rs` 创建)
///
/// # 返回值
///
/// - `true`: 多线程环境，需要加锁
/// - `false`: 单线程环境，跳过所有锁操作
pub(crate) fn is_mt() -> bool {
    // 等价于 C 的 MT 宏: #define MT (libc.need_locks)
    // need_locks != 0 表示多线程模式已启动，需加锁保护。
    // 注意: need_locks 默认为 -1（安全默认值 = 需要锁），
    // 等线程子系统初始化后才会根据实际线程数调整。
    is_mt_inner()
}

/// is_mt 的内部实现，通过 cfg 处理测试/生产模式的差异。
/// 在测试模式或集成测试模式下，始终返回 false（假定单线程）。
fn is_mt_inner() -> bool {
    // Safety: __libc 为 static mut，但 need_locks 仅在启动阶段写入，
    // 之后为只读访问，且 i8 读取在支持的平台上是原子操作。
    use crate::import::__libc;
    unsafe { __libc.need_locks != 0 }
}

// ============================================================================
// 断言宏
// ============================================================================

/// mallocng 内部一致性检查断言。
///
/// 默认行为（非 test 模式）：断言失败时调用 `rusl_internal::atomic::a_crash()` 直接终止进程。
/// 此行为不受 `debug_assert!` 的 `debug-only` 限制——分配器内部的不变式违反
/// 意味着堆已损坏，继续执行可能导致数据丢失或安全漏洞。
///
/// 在 test 模式下：使用标准 `assert!` 宏，通过 panic 提供可捕获的失败信息，
/// 便于单元测试验证不变量。
///
/// # 示例
///
/// ```ignore
/// malloc_assert!(ctx.secret == meta_area.check, "元数据校验失败");
/// malloc_assert!(p as usize % PAGE_SIZE == 0);
/// ```
#[allow(unused_macros)]
macro_rules! malloc_assert {
    ($cond:expr $(,)?) => {
        if cfg!(test) {
            assert!($cond);
        } else {
            if !$cond {
                // 直接触发 SIGSEGV 终止进程，无需依赖 crate::atomic 模块
                // （该模块在 test 模式下不可用）
                unsafe { core::ptr::write_volatile(0 as *mut u8, 0); }
                #[allow(unconditional_recursion)]
                fn loop_forever() -> ! { loop_forever(); }
                loop_forever();
            }
        }
    };
    ($cond:expr, $($arg:tt)*) => {
        if cfg!(test) {
            assert!($cond, $($arg)*);
        } else {
            if !$cond {
                unsafe { core::ptr::write_volatile(0 as *mut u8, 0); }
                #[allow(unconditional_recursion)]
                fn loop_forever() -> ! { loop_forever(); }
                loop_forever();
            }
        }
    };
}

pub(crate) use malloc_assert;
use rusl_internal::do_syscall;

// ============================================================================
// futex 辅助函数 (MallocLock 的底层阻塞原语)
// ============================================================================

/// futex(FUTEX_WAIT|FUTEX_PRIVATE) — 阻塞等待锁值变为非 val。
///
/// 当 lock 的当前值不等于 `val` 时立即返回（规避丢失唤醒）；
/// 否则挂起调用线程直到被 futex_wake 唤醒或锁值改变。
///
/// # Safety
///
/// - `addr` 必须指向一个有效的 `AtomicI32`
/// - 调用环境必须能安全陷入内核（即不在信号处理上下文中持有锁时调用）
unsafe fn futex_wait(addr: *const AtomicI32, val: i32) {
    do_syscall!(
        SYS_FUTEX,
        addr as i64,
        (FUTEX_WAIT | FUTEX_PRIVATE) as i64,
        val as i64,
        0 // timeout = NULL (无限等待)
    );
}

/// futex(FUTEX_WAKE|FUTEX_PRIVATE, cnt) — 唤醒最多 `cnt` 个等待者。
///
/// # Safety
///
/// - `addr` 必须指向一个有效的 `AtomicI32`
unsafe fn futex_wake(addr: *const AtomicI32, cnt: i32) {
    do_syscall!(
        SYS_FUTEX,
        addr as i64,
        (FUTEX_WAKE | FUTEX_PRIVATE) as i64,
        cnt as i64
    );
}

// ============================================================================
// 锁类型: MallocLock
// ============================================================================

/// musl mallocng 分配器的全局互斥锁。
///
/// 基于 futex 的自旋锁实现，内部使用 `AtomicI32` 作为锁状态。
/// 锁状态语义：`0` = 未锁定，`1` = 已锁定（有竞争时通过 futex 等待）。
///
/// 在单线程模式下（`is_mt() == false`），所有锁操作退化为空操作，
/// 以实现零开销的线程安全抽象。
///
/// # 不变量 (INV-LOCK-01)
///
/// `rdlock()`/`wrlock()` 与 `unlock()` 必须成对出现。在 `fork()` 子进程中，
/// `resetlock()` 必须在首次锁操作前被调用。
///
/// # 不变量 (INV-LOCK-03)
///
/// `sizeof(MallocLock) == sizeof(AtomicI32)` (4 字节)，与 C 原版的 `int[1]` 大小一致。
pub(crate) struct MallocLock {
    /// 锁状态: 0 = 未锁定, 1 = 已锁定
    lock: AtomicI32,
}

impl MallocLock {
    /// 创建初始未锁定状态的锁实例。
    ///
    /// 使用 `const fn` 允许在静态初始化上下文中使用。
    ///
    /// 注意: `MALLOC_LOCK` 静态实例使用结构体字面量直接初始化
    /// (`MallocLock { lock: AtomicI32::new(0) }`)，因为 `AtomicI32::new(0)` 是 `const` 稳定的。
    /// 此函数保留作为完整的构造器 API 供模块外部使用。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// pub(crate) static MALLOC_LOCK: MallocLock = MallocLock::new();
    /// ```
    pub(crate) fn new() -> Self {
        MallocLock {
            lock: AtomicI32::new(0),
        }
    }

    /// 读锁（排他锁，与写锁实现相同）。
    ///
    /// 多线程模式下获取排他锁，单线程模式下为空操作。
    ///
    /// 由于 `RDLOCK_IS_EXCLUSIVE = true`，读写锁之间无区别，
    /// 均获取排他互斥锁。
    ///
    /// # 锁定策略
    ///
    /// 1. 检查 `is_mt()`：若单线程则直接返回
    /// 2. 使用 CAS 循环尝试将锁从 0 置为 1
    /// 3. 若 CAS 失败（锁已被持有），通过 futex(FUTEX_WAIT) 阻塞等待
    pub(crate) fn rdlock(&self) {
        // 单线程模式下跳过所有锁操作
        if !is_mt() {
            return;
        }

        // musl __lock 算法: INT_MIN 作锁标志位 + 低比特位同辰计数
        //
        // 锁状态语义 (x = self.lock 的值):
        //   x == 0:                         未锁定，无线程在临界区
        //   x < 0  (即 INT_MIN 被置位):    已锁定，同辰量 = x - INT_MIN
        //   x > 0:                         未锁定但有 x 个同辰线程（理论上仅出现在竞争中间态）
        //
        // fast-path: CAS(0, INT_MIN + 1) —— 尝试以"仅自身一个持有者"状态获取锁
        let mut current = self
            .lock
            .compare_exchange(0, INT_MIN + 1, Ordering::Acquire, Ordering::Relaxed)
            .unwrap_or_else(|v| v);
        if current == 0 {
            return; // fast-path 成功
        }

        // medium-path: 最多自旋 10 次，尝试在原地递增同辰计数
        for _ in 0..10 {
            if current < 0 {
                current -= INT_MIN + 1;
            }
            // current >= 0 此时表示同辰计数
            let val = INT_MIN + (current + 1);
            match self
                .lock
                .compare_exchange(current, val, Ordering::Acquire, Ordering::Relaxed)
            {
                Ok(_) => return, // 获得锁
                Err(v) => current = v,
            }
        }

        // heavy-path: 自旋失败，将自己标记为等待者，然后进入 futex 等待循环
        // fetch_add(1) 递增同辰寄存器（把自己纳入计数）
        current = self.lock.fetch_add(1, Ordering::Relaxed) + 1;

        loop {
            // 仅当锁被他人持有时才进入 futex 等待（确保有人会唤醒我们）
            if current < 0 {
                unsafe {
                    futex_wait(&self.lock, current);
                }
                current -= INT_MIN + 1;
            }
            // current > 0: 同辰计数包含我们自己，尝试获取锁
            match self.lock.compare_exchange(
                current,
                INT_MIN + current,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return, // 获得锁
                Err(v) => current = v,
            }
        }
    }

    /// 写锁（排他锁，与读锁实现相同）。
    ///
    /// 多线程模式下获取排他锁，单线程模式下为空操作。
    ///
    /// 由于 `RDLOCK_IS_EXCLUSIVE = true`，此方法与 `rdlock()` 行为完全一致。
    /// 保留独立方法名是为了代码可读性和将来可能的读写锁分离。
    pub(crate) fn wrlock(&self) {
        // 由于 RDLOCK_IS_EXCLUSIVE = true，wrlock 与 rdlock 实现完全一致：
        // 都是获取排他锁。
        self.rdlock()
    }

    /// 释放锁。
    ///
    /// 多线程模式下释放排他锁并唤醒等待者，单线程模式下为空操作。
    ///
    /// # 释放策略
    ///
    /// 1. 检查 `is_mt()`：若单线程则直接返回
    /// 2. 将锁从 1 置为 0（使用 store + fence 保证内存序）
    /// 3. 通过 futex(FUTEX_WAKE) 唤醒一个等待者
    ///
    /// # Safety
    ///
    /// 调用者必须确保在持有锁的同一线程中调用 `unlock()`。
    /// 从非持有者线程释放锁会导致未定义行为。
    pub(crate) fn unlock(&self) {
        // 注意: 与 __lock 不同，musl 的 __unlock 不检查 need_locks。
        // 原因: 若 need_locks=0，锁从未被获取 (l[0]=0)，下面的 l[0] < 0 检查自然通过。
        //
        // 算法:
        //   1. 若锁未处于"已锁定"状态 (l[0] >= 0)，什么都不做（快速返回）
        //   2. fetch_add(-(INT_MIN+1)): 清除 INT_MIN 锁标志并递减同辰计数
        //   3. 若原值 != INT_MIN+1 (仅 1 个持有者)，说明有其他线程在等待，wake 一个

        let val = self.lock.load(Ordering::Relaxed);
        if val >= 0 {
            return; // 未锁定或仅同辰状态，无需释放
        }

        // 原子解除锁定: 减去 (INT_MIN + 1) = 清除标志 + 递减计数
        let prev = self.lock.fetch_add(-(INT_MIN + 1), Ordering::Release);

        // 若之前有多个持有者/等待者 (prev != INT_MIN + 1)，
        // 则唤醒一个等待者
        if prev != INT_MIN + 1 {
            unsafe {
                futex_wake(&self.lock, 1);
            }
        }
    }

    /// 锁升级（当前为空操作）。
    ///
    /// 保留接口以备将来区分读写锁实现。
    /// 由于 `RDLOCK_IS_EXCLUSIVE = true`，读锁已是排他的，无需升级。
    ///
    /// 如果将来实现了真正的读写锁（RDLOCK_IS_EXCLUSIVE = false），
    /// 此方法将从共享读锁升级为排他写锁。
    pub(crate) fn upgradelock(&self) {
        // 由于 RDLOCK_IS_EXCLUSIVE = true，读锁已是排他的，无需升级。
        // 保留此方法仅为 API 完整性，将来若实现真正的读写锁时可填充实际逻辑。
    }

    /// 重置锁状态，将锁强制归零。
    ///
    /// 仅在 `fork()` 后的子进程中调用（单线程上下文，父进程的锁状态无效）。
    /// 在子进程中，只有当前线程存在，但锁可能保留了父进程某线程的已锁定状态，
    /// 因此需要强制重置。
    ///
    /// # Safety
    ///
    /// - 必须在确认单线程环境（子进程 fork 后）中调用
    /// - 调用时不得有任何线程持有该锁
    /// - 此方法绕过了正常的获取/释放配对协议
    pub(crate) unsafe fn resetlock(&self) {
        // 等价于 C 原版的: __malloc_lock[0] = 0;
        // 仅在 fork() 子进程中调用，父进程的锁状态在子进程中无效。
        // 使用 Release 语义确保所有在 fork 前对堆的修改对子进程可见。
        self.lock.store(0, Ordering::Release);
    }

    /// atfork 回调处理。
    ///
    /// 根据 `who` 参数执行相应操作，由 `pthread_atfork()` 机制在 `fork()` 前后调用：
    ///
    /// - `who < 0` (prepare): 获取锁，阻止其他线程在 fork 期间修改堆。
    ///   在 fork() 之前调用，确保 fork 时堆处于一致状态。
    /// - `who == 0` (parent): 释放 prepare 阶段获取的锁。
    ///   在 fork() 返回后（父进程）调用，恢复正常操作。
    /// - `who > 0` (child): 强制重置锁状态。
    ///   在 fork() 返回后（子进程）调用，因为子进程只有当前线程，
    ///   但锁状态是从父进程拷贝的（可能处于已锁定状态）。
    ///
    /// # 参数
    ///
    /// - `who`: atfork 阶段标识，遵循 musl/pthread 的约定
    ///
    /// # 依赖
    ///
    /// - 单线程检测: 通过 `is_mt()` 判断是否需要实际加锁
    pub(crate) fn atfork(&self, who: c_int) {
        // 等价于 C 原版的 malloc_atfork():
        //   if (who<0) rdlock();
        //   else if (who>0) resetlock();
        //   else unlock();
        if who < 0 {
            self.rdlock(); // prepare: fork 前获取锁
        } else if who > 0 {
            unsafe {
                self.resetlock(); // child: fork 子进程重置锁
            }
        } else {
            self.unlock(); // parent: fork 后释放锁
        }
    }
}

// ============================================================================
// 全局锁实例
// ============================================================================

/// musl mallocng 全局互斥锁实例。
///
/// 使用结构体字面量直接初始化（`MallocLock { lock: AtomicI32::new(0) }`），
/// 等价于 C 原版的 `int __malloc_lock[1]` 全局变量初始化为 `{0}`。
///
/// `AtomicI32::new(0)` 在 Rust 中是 const-stable 操作，
/// 因此可以在 `static` 初始化上下文中直接使用。
///
/// # 跨模块可见性
///
/// `pub(crate)`: 整个 rusl mallocng 子系统内可见。
/// 所有 mallocng 子模块（malloc、free、realloc、aligned_alloc 等）
/// 通过此静态实例访问全局锁。
///
/// # 线程安全
///
/// 内部使用 `AtomicI32`，通过 `&self`（不可变引用）安全地进行内部可变性操作，
/// 避免了对 `static mut` 和 `unsafe` 块的需求。
pub(crate) static MALLOC_LOCK: MallocLock = MallocLock {
    lock: AtomicI32::new(0),
};

// ============================================================================
// atfork 对外回调函数
// ============================================================================

/// musl atfork 回调函数。由 `pthread_atfork()` 机制在 `fork()` 前后调用。
///
/// 此函数必须保持 C ABI 兼容签名，因为外部 C 代码（`pthread_atfork` 实现）
/// 通过符号名 `__malloc_atfork` 查找并调用此回调。
///
/// 函数体为薄封装，实际逻辑委托给 `MALLOC_LOCK.atfork(who)`。
///
/// # C ABI 兼容性
///
/// - 导出符号名: `__malloc_atfork`
/// - 函数签名: `extern "C" fn(c_int)`
/// - 参数语义: `who < 0` = prepare, `who == 0` = parent, `who > 0` = child
///
/// # 调用时机
///
/// | who    | 阶段     | 调用时机             | 操作            |
/// |--------|----------|----------------------|-----------------|
/// | `< 0`  | prepare  | fork() 之前          | 获取锁          |
/// | `== 0` | parent   | fork() 返回后(父进程) | 释放锁          |
/// | `> 0`  | child    | fork() 返回后(子进程) | 强制重置锁状态  |
#[no_mangle]
pub extern "C" fn __malloc_atfork(who: c_int) {
    MALLOC_LOCK.atfork(who);
}

// ============================================================================
// 随机密钥生成
// ============================================================================

/// ELF 辅助向量类型: 内核提供的 16 字节随机种子地址。
/// 由 Linux 内核在进程启动时通过 auxv 传递给用户空间。
#[cfg(target_os = "linux")]
const AT_RANDOM: usize = 25;

/// 为分配器生成一个进程生命期内**固定的随机密钥**。
///
/// 用于 `meta_area.check` 字段，防止元数据伪造攻击。
/// 通过组合两个独立熵源降低可预测性风险。
///
/// # 算法（两步混合）
///
/// **Step 1 — 栈地址熵源**:
/// 获取栈上局部变量的地址（受 ASLR 影响），乘以经典 LCG 乘数进行扩散。
///
/// **Step 2 — 内核随机种子熵源**:
/// 遍历 auxv 查找 `AT_RANDOM` 条目，读取内核提供的 16 字节随机种子。
/// 内核熵源（由内核在 execve 时生成）质量高于栈地址，直接覆盖 Step 1 的结果。
///
/// # 前置条件
///
/// - `crate::runtime::auxv()` 已初始化（辅助向量在进程启动时由动态链接器设置）
///   (TODO: `crate::runtime` 模块尚未实现，待 `src/runtime.rs` 创建)
///
/// # 返回值
///
/// - 一个 64 位无符号随机值，在进程生命期内保持不变
///
/// # 调用上下文
///
/// 仅在 `alloc_meta()` 初始化路径中被调用一次，结果存入 `ctx.secret`。
///
/// # 不变量 (INV-SECRET-01)
///
/// `ctx.secret` 在进程生命期内保持不变，且 `meta_area.check` 必须始终等于
/// `ctx.secret`。此不变量由 `get_meta()` 中的断言检查保证。
/// 从内核 at_random 熵源中提取随机密钥（生产模式）。
///
/// 遍历 auxv 查找 AT_RANDOM 条目，读取内核提供的 16 字节随机种子的高 8 字节。
/// 若未找到 AT_RANDOM（例如 auxv 未初始化），回退到栈地址熵源。
#[cfg(not(test))]
fn get_kernel_random_secret(stack_secret: u64) -> u64 {
    use crate::import::__libc;
    // Safety: __libc.auxv 在进程启动时由 crt 初始化，之后只读。
    let auxv = unsafe { __libc.auxv };
    if auxv.is_null() {
        return stack_secret;
    }

    let mut i = 0;
    loop {
        unsafe {
            let entry_type = *auxv.add(i);
            if entry_type == 0 {
                break; // AT_NULL — auxv 结束
            }
            if entry_type == AT_RANDOM {
                // auxv[i+1] 指向 16 字节随机种子缓冲区
                // C 原版: memcpy(&secret, (char *)libc.auxv[i+1]+8, sizeof secret)
                // 读取第 9-16 字节（高 8 字节）
                let random_ptr = *auxv.add(i + 1) as *const u8;
                return core::ptr::read_unaligned(random_ptr.add(8) as *const u64);
            }
            i += 2;
        }
    }

    stack_secret
}

/// 测试模式/集成测试下的随机密钥生成: 仅使用栈地址熵源。
#[cfg(test)]
fn get_kernel_random_secret(stack_secret: u64) -> u64 {
    // 测试模式下 crate::libc 不可用，仅使用栈地址作为熵源。
    stack_secret
}

pub(crate) fn get_random_secret() -> u64 {
    // Step 1: 栈地址熵源（受 ASLR 影响）
    // 获取栈上局部变量地址，乘以经典 LCG 乘数 1103515245 进行扩散。
    let stack_var: u64 = 0;
    let stack_addr = core::ptr::addr_of!(stack_var) as u64;
    let secret = stack_addr.wrapping_mul(1103515245);

    // Step 2: 内核随机种子熵源（质量更高，直接覆盖 Step 1 结果）
    get_kernel_random_secret(secret)
}

// ============================================================================
// 向后兼容的全局函数别名 (过渡期使用)
// ============================================================================

/// 获取全局 malloc 锁（排他锁）的便捷函数。
///
/// 委托给 `MALLOC_LOCK.wrlock()`。
/// 保留此函数以支持从旧版 API 渐进迁移的模块。
/// 新代码应直接使用 `MALLOC_LOCK.wrlock()`。
pub(crate) fn wrlock() {
    MALLOC_LOCK.wrlock();
}

/// 释放全局 malloc 锁的便捷函数。
///
/// 委托给 `MALLOC_LOCK.unlock()`。
/// 保留此函数以支持从旧版 API 渐进迁移的模块。
/// 新代码应直接使用 `MALLOC_LOCK.unlock()`。
pub(crate) fn unlock() {
    MALLOC_LOCK.unlock();
}

/// 检测当前进程是否处于多线程模式（向后兼容别名）。
///
/// 委托给 `is_mt()`。保留以支持从旧名称"is_multi_threaded"迁移的代码。
/// 新代码应直接使用 `is_mt()`。
pub(crate) fn is_multi_threaded() -> bool {
    is_mt()
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::mem;
    use core::sync::atomic::Ordering;

    // ---------------------------------------------------------------------------
    // 编译时常量测试
    // ---------------------------------------------------------------------------

    test!("test_use_madv_free_default" {
        // spec: USE_MADV_FREE 默认值为 false
        assert!(!USE_MADV_FREE);
    });

    test!("test_rdlock_is_exclusive" {
        // spec: RDLOCK_IS_EXCLUSIVE 为 true
        // 在 mallocng 场景下，读写者无并发收益
        assert!(RDLOCK_IS_EXCLUSIVE);
    });

    // ---------------------------------------------------------------------------
    // MADV 常量值测试
    // ---------------------------------------------------------------------------

    test!("test_madv_free_value" {
        // MADV_FREE 在 Linux 内核中恒为 8
        assert_eq!(MADV_FREE, 8);
    });

    test!("test_madv_dontneed_value" {
        // MADV_DONTNEED 在 Linux 内核中恒为 4
        assert_eq!(MADV_DONTNEED, 4);
    });

    // ---------------------------------------------------------------------------
    // mmap/mprotect 常量值测试
    // ---------------------------------------------------------------------------

    test!("test_prot_constants" {
        assert_eq!(PROT_NONE, 0);
        assert_eq!(PROT_READ, 1);
        assert_eq!(PROT_WRITE, 2);
        assert_eq!(PROT_EXEC, 4);
        assert_eq!(PROT_READ | PROT_WRITE, 3);
        assert_eq!(PROT_READ | PROT_WRITE | PROT_EXEC, 7);
    });

    test!("test_map_constants" {
        #[cfg(target_os = "linux")]
        {
            assert_eq!(MAP_PRIVATE, 2);
            assert_eq!(MAP_SHARED, 1);
            assert_eq!(MAP_ANONYMOUS, 32);
            assert_eq!(MAP_FIXED, 16);
        
    }
    });

    test!("test_map_failed_value" {
        // MAP_FAILED 应为 (void*)-1
        assert_eq!(MAP_FAILED, (-1isize) as *mut c_void);
    });

    test!("test_map_failed_is_not_null" {
        // MAP_FAILED 应与 NULL 严格区分
        // NULL 是合法的 mmap 返回值（表示"让内核选择地址"时的提示值）
        assert!(!MAP_FAILED.is_null());
    });

    // ---------------------------------------------------------------------------
    // 函数签名编译测试 (验证接口可编译)
    // ---------------------------------------------------------------------------

    test!("test_syscall_signatures_compile" {
        // 测试所有系统调用封装函数的类型签名可编译
        // brk
        let _brk_fn: unsafe fn(usize) -> usize = brk;

        // mmap
        let _mmap_fn: unsafe fn(*mut c_void, usize, c_int, c_int, c_int, i64) -> *mut c_void =
            mmap;

        // madvise
        let _madvise_fn: unsafe fn(*mut c_void, usize, c_int) -> c_int = madvise;

        // mremap
        let _mremap_fn: unsafe fn(*mut c_void, usize, usize, c_int, *mut c_void) -> *mut c_void =
            mremap;

        // munmap
        let _munmap_fn: unsafe fn(*mut c_void, usize) -> c_int = munmap;

        // mprotect
        let _mprotect_fn: unsafe fn(*mut c_void, usize, c_int) -> c_int = mprotect;
    });

    test!("test_runtime_fn_signatures_compile" {
        // 测试运行时配置函数的类型签名可编译
        // disable_aligned_alloc
        let _daa_fn: fn() -> bool = disable_aligned_alloc;

        // is_mt
        let _mt_fn: fn() -> bool = is_mt;

        // page_size
        let _ps_fn: fn() -> usize = page_size;

        // get_random_secret
        let _grs_fn: fn() -> u64 = get_random_secret;
    });

    test!("test_malloc_assert_true_no_message" {
        // 测试 malloc_assert! 不带消息的语法 — 条件为 true 时不 panic
        malloc_assert!(true);
    });

    test!("test_malloc_assert_trailing_comma" {
        // 测试 malloc_assert! 尾随逗号语法（Rust 允许的语法糖）
        malloc_assert!(true, "带尾随逗号",);
    });
}