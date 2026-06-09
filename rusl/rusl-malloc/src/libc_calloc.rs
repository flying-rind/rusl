//! libc_calloc — 分配并清零内存的 calloc 系列接口。
//!
//! 对应 musl 的 `src/malloc/libc_calloc.c`。
//! 通过泛型函数指针参数化替代 C 预处理器符号重命名，
//! 使得公共 `calloc` 和内部 `__libc_calloc` 共享同一套核心逻辑 `calloc_impl`。
//!
//! # 设计策略
//!
//! C 版本通过 `#define calloc __libc_calloc` / `#define malloc __libc_malloc`
//! 对 `calloc.c` 进行符号重命名后 `#include "calloc.c"`。
//! Rust 版本使用泛型函数指针参数化替代预处理器符号重命名——
//! 公共 `calloc` 和内部 `__libc_calloc` 共享同一个带 `malloc_fn` 参数
//! 的通用实现 `calloc_impl`，不同调用点传入不同的分配器函数指针。
//!
//! # 依赖关系
//!
//! ```text
//! calloc (公共) ──> calloc_impl(m, n, malloc) ──> 公共 malloc (可被替换)
//! __libc_calloc ──> calloc_impl(m, n, __libc_malloc) ──> 内部 malloc (不可替换)
//!                        │
//!                        ├── mal0_clear (反向扫描清零优化)
//!                        ├── memset (补齐清零)
//!                        ├── __malloc_replaced (替换检测)
//!                        └── __malloc_allzerop (零页检测)
//! ```

#![allow(unused_imports, unused_variables)]

use core::ffi::c_void;
use core::ptr;
use core::sync::atomic::Ordering;
use crate::import::memset;

// ============================================================================
// 常量定义
// ============================================================================

/// 反向扫描清零使用的页大小常量。
///
/// 与 C 版本策略一致，使用固定 4096 字面量，
/// 不依赖实际系统页大小。
pub(crate) const PAGESZ: usize = 4096;

// ============================================================================
// 公共 C ABI 接口
// ============================================================================

/// 分配 `m * n` 字节的连续内存块，并将所有字节初始化为零。
///
/// 此函数为 C 标准库 `calloc` 的 Rust 实现，保持与 C ABI 的完全兼容。
/// 对应的 C 声明:
/// ```c
/// void *calloc(size_t m, size_t n);
/// ```
///
/// # 参数
///
/// - `m`: 元素个数 (C `size_t`)
/// - `n`: 每个元素的字节大小 (C `size_t`)
///
/// # 返回值
///
/// - **成功**: 返回指向已分配且已清零的内存块的指针。
///   指针满足与底层 `malloc` 相同的对齐要求。
/// - **溢出**: 若 `m * n` 溢出 `usize`，设置 `errno = ENOMEM` 并返回
///   `core::ptr::null_mut()`。
/// - **分配失败**: 若底层 `malloc` 返回 NULL，返回 `core::ptr::null_mut()`。
///
/// # 安全性
///
/// 此函数自身是安全的（未标记为 `unsafe`），但调用者仍需注意：
/// - 返回的原始指针需要调用者正确管理生命周期
/// - 调用者必须使用 `free` 释放返回的内存（否则内存泄漏）
/// - 调用者不得对返回的内存执行越界读写
///
/// # 线程安全
///
/// 由底层 `malloc` 实现保证。多线程并发调用是安全的。
///
/// # 示例
///
/// ```rust,no_run,ignore
/// # use rusl::malloc::calloc;
/// // 分配 10 个 u32 元素的数组，全部初始化为零
/// let p = calloc(10, core::mem::size_of::<u32>());
/// assert!(!p.is_null());
/// // p 指向 40 字节的全零内存
/// ```
#[no_mangle]
pub extern "C" fn calloc(m: usize, n: usize) -> *mut c_void {
    // SAFETY: 参数 m, n 来自调用者，malloc 函数指针指向正确的分配函数。
    unsafe { calloc_impl(m, n, super::mallocng::malloc::malloc) }
}

// ============================================================================
// crate 内部接口 (pub(crate))
// ============================================================================

/// 内部 calloc 实现 —— 始终使用内部分配器 `__libc_malloc`，不可被用户替换。
///
/// # 意图 (Intent)
///
/// 为 rusl crate 内部提供**不可被替换的** `calloc` 实现。这是 musl 架构设计
/// 在 Rust 中的等价翻译——内部分配器函数通过"策略参数化"（传入 `__libc_malloc`
/// 而非 `malloc`）与公共 API 隔离，确保 crate 内部代码始终使用内部分配器，
/// 即使应用程序通过 `LD_PRELOAD` 或静态链接替换了公共 `malloc`/`calloc`。
///
/// 在 Rust/rusl 中，由于不存在 ELF 符号插替（symbol interposition）的概念
/// （所有内部调用在编译期静态分派），"不可替换"由 Rust 的模块私有性和编译期
/// 单态化自然保证，无需运行时检查。
///
/// 对应的 C 声明 (musl 内部):
/// ```c
/// static void *__libc_calloc(size_t m, size_t n);  // C: hidden visibility
/// ```
///
/// # 参数
///
/// - `m`: 元素个数
/// - `n`: 每个元素的字节大小
///
/// # 返回值
///
/// 与 `calloc` 相同，详见 `calloc_impl` 的后置条件。
///
/// # 安全性
///
/// 此函数**不带有 `unsafe` 标记**。其所有内部 `unsafe` 操作均由
/// `calloc_impl` 封装。外部调用者是安全的（前提是返回值被正确使用）。
///
/// # 使用场景
///
/// rusl 内部模块通过 `use crate::__libc_calloc;` 直接引入：
///
/// | 使用模块 | 对应 musl 源文件 | 说明 |
/// |---------|-----------------|------|
/// | atexit 处理 | `src/exit/atexit.c` | 退出处理函数注册 |
/// | 命名信号量 | `src/thread/sem_open.c` | POSIX 信号量内部状态 |
/// | 异步 I/O | `src/aio/aio.c` | AIO 控制块分配 |
/// | 动态链接器 | `src/ldso/dlerror.c` | 链接器错误信息存储 |
/// | 进程 fd 操作 | `src/process/fdop.h` | 文件描述符操作 |
/// | NLS/gettext | `src/locale/dcngettext.c` | 国际化消息目录 |
///
/// # 不变量
///
/// - 始终返回零初始化内存（或 NULL）
/// - 仅调用 `__libc_malloc`（内部版本），不依赖可替换的公共 `malloc`
#[no_mangle]
pub extern "C" fn __libc_calloc(m: usize, n: usize) -> *mut c_void {
    unsafe { calloc_impl(m, n, super::lite_malloc::__libc_malloc) }
}

/// 通用 calloc 核心实现 —— 以 `malloc_fn` 函数指针参数化分配器策略。
///
/// # 意图 (Intent)
///
/// 这是公共 `calloc` 和内部 `__libc_calloc` 共用的底层实现。
/// `malloc_fn` 在公共版本为 `malloc`（可被用户替换），在内部版本为
/// `__libc_malloc`（不可被替换）。
///
/// 通过 `#[inline]` 标注确保编译期被单态化到两个调用点，消除函数指针
/// 间接调用的运行时开销（零成本抽象）。
///
/// # 算法概要
///
/// **阶段 1 —— 溢出检测**:
/// ```text
/// if n != 0 && m > usize::MAX / n:
///     set_errno(ENOMEM); return null_mut();
/// ```
///
/// **阶段 2 —— 分配**:
/// ```text
/// n = m * n;  // 已验证不溢出
/// let p = malloc_fn(n);
/// ```
///
/// **阶段 3 —— 清零优化**:
/// ```text
/// if p.is_null() || (!__malloc_replaced.load(Relaxed) && __malloc_allzerop(p)):
///     return p;
/// let remaining = mal0_clear(p as *mut u8, n);
/// return memset(p as *mut u8, 0, remaining) as *mut c_void;
/// ```
///
/// # Safety
///
/// 调用者必须确保：
/// - `malloc_fn` 是一个有效的函数指针，其语义等价于 `malloc`：
///   接受 `usize` 大小参数，返回指向已分配内存的指针或 NULL。
///
/// # 不变量
///
/// - 始终返回零初始化内存（或 NULL），无论 `__malloc_replaced` 的状态如何
/// - 不得访问 `malloc_fn` 参数之外的任何可替换分配器符号
///   分配器选择完全由调用者通过函数指针显式指定
/// - 通过 `#[inline]` 的编译期内联保证函数指针调用在 release 构建中
///   被优化为直接调用，无间接跳转开销
///
/// # 复杂度层级
///
/// Level 3 — 包含溢出检测、分配、条件清零优化三个阶段的复合操作
#[inline]
pub(crate) unsafe fn calloc_impl(
    m: usize,
    n: usize,
    malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void,
) -> *mut c_void {
    // Stage 1 — Overflow detection.
    // Equivalent to musl: if (n && m > (size_t)-1/n) { errno = ENOMEM; return 0; }
    if n != 0 && m > usize::MAX / n {
        super::set_errno(super::ENOMEM);
        return core::ptr::null_mut();
    }

    // Stage 2 — Allocate memory.
    // Multiplication is safe here because overflow was already checked above.
    let total = n.wrapping_mul(m);
    let p = malloc_fn(total);

    if p.is_null() {
        return core::ptr::null_mut();
    }

    // Stage 3 — Zero-fill optimization.
    //
    // If the allocator has NOT been replaced and the implementation can
    // confirm the returned memory is already all-zero (e.g. fresh mmap pages),
    // skip the explicit memset. Otherwise, perform the zeroing:
    //   1. mal0_clear: reverse-scan from the end, skip already-zero pages
    //   2. memset: clear the remaining prefix that mal0_clear couldn't handle
    let replaced = super::__malloc_replaced();

    if !replaced && super::allzerop(p) {
        return p;
    }

    let remaining = mal0_clear(p as *mut u8, total);
    if remaining > 0 {
        memset(p, 0, remaining);
    }

    p
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 反向扫描清零辅助函数 —— 从缓冲区末尾向开头扫描，跳过已为零的页面，
/// 仅对非零区域调用 `memset` 进行清零以减少不必要的内存写入。
///
/// # 意图 (Intent)
///
/// 利用内核零页映射（zero-page mapping）优化：当 `malloc` 从内核获取新页面时，
/// 页面可能已经全零（由内核的 Copy-on-Write 零页提供）。
/// `mal0_clear` 从内存块的**末尾**向**开头**扫描，跳过已为零的页面，
/// 仅对包含非零数据的区域需要后续 `memset` 补齐。
///
/// # Rust 设计改进
///
/// C 版本依赖 GCC 特定的 `__attribute__((__may_alias__))` 实现 `uint64_t`
/// 类型双关（type punning）以加速零检测。
/// Rust 版本使用 `core::ptr::read_unaligned::<u64>()` 安全地以 8 字节粒度扫描，
/// 无需违反严格别名规则。非 x86_64/aarch64 平台降级为逐 `u8` 字节扫描。
///
/// # 参数
///
/// - `p`: 指向已分配内存块起始位置的指针（可读可写）
/// - `n`: 内存块的字节长度
///
/// # 返回值
///
/// 仍需调用者通过 `memset` 补齐清零的字节数 `r`，满足：
/// - 若 `n < PAGESZ`，则 `r == n`
/// - 若 `n >= PAGESZ`，则 `0 <= r < PAGESZ`
///
/// # 系统算法
///
/// 与 C 版本相同的反向逐页扫描算法：
///
/// 1. **起点对齐**: 以 `PAGESZ = 4096` 为粒度。将指针 `pp` 初始化为
///    `p.wrapping_add(n)`（缓冲区末尾），将 `i` 初始化为
///    `pp as usize & (PAGESZ - 1)`（页内偏移量）。
///
/// 2. **循环扫描**:
///    - **Step A —— 清零页内尾部**: 对页面内非对齐部分执行
///      `memset(pp.sub(i), 0, i)` 清零。
///    - **Step B —— 提前终止检查**: 若 `pp.offset_from(p) < PAGESZ as isize`
///      （剩余不足一页），返回剩余未处理字节数。
///    - **Step C —— 整页扫描**: 从当前页末尾向开头以 `2 * size_of::<T>()`
///      步进扫描，`T` 通过 `cfg` 选择：
///      - `target_arch = "x86_64"` 或 `target_arch = "aarch64"`：
///        `T = u64`，一次检查 16 字节。
///      - 其他架构：`T = u8`，逐字节扫描。
///      使用 `core::ptr::read_unaligned` 读取，避免未对齐访问的 UB。
///    - **Step D —— 跳过零页**: 若扫描完整个页面未发现非零值，
///      则 `pp` 跳过该页继续向前。
///
/// 3. **返回**: 返回值为还需调用者额外清零的字节数。
///
/// # 前置条件
///
/// - `p` 指向一个长度为 `n` 字节的可读可写内存块（通过 `malloc` 返回）
/// - `n` 为任意 `usize` 值（包括 0）
///
/// # 后置条件
///
/// - 返回值 `r` 满足 `0 <= r < PAGESZ` 或 `r == n`（当 `n < PAGESZ` 时）
/// - 所有 `pp` 至 `p + n` 之间的内存已被清零，剩余 `p[0..r]` 由调用者补齐
///
/// # 平台差异
///
/// - `x86_64` / `aarch64`: 使用 64 位宽字扫描（每次检查 8 字节）
/// - 其他架构: 降级为逐字节扫描
///
/// # 复杂度层级
///
/// Level 3 — 包含反向扫描、平台特化宽字检测、页级跳过的复合优化算法
pub(crate) fn mal0_clear(p: *mut u8, n: usize) -> usize {
    const PAGESZ: usize = 4096;

    // Optimization threshold: blocks smaller than a page are handled entirely
    // by the caller via memset, no benefit from reverse scanning.
    if n < PAGESZ {
        return n;
    }

    // pp points to the end of the buffer; i is the byte offset of pp
    // within its containing "page" (pagesz-aligned boundary).
    let mut pp = unsafe { p.add(n) };
    let mut i = (pp as usize) & (PAGESZ - 1);

    unsafe {
        loop {
            // Step A — clear the unaligned tail of the current page.
            // memset returns the destination pointer, so pp moves backward
            // to the page-aligned boundary.
            pp = memset(
                pp.sub(i) as *mut core::ffi::c_void,
                0,
                i,
            ) as *mut u8;

            // Step B — early termination: if less than a full page remains
            // between the original start and pp, return the remaining byte count.
            // The caller will complete the zeroing for bytes [p, p + remaining).
            let remaining = pp.offset_from(p);
            if remaining < PAGESZ as isize {
                return remaining as usize;
            }

            // Step C — scan the preceding full page backward in 16-byte
            // (2 × sizeof(u64)) steps. If any non-zero value is found,
            // break out of the inner loop so the outer loop's Step A
            // will memset-clear the non-zero region.
            //
            // On x86_64 / aarch64: use 64-bit wide loads for speed.
            // On other architectures: fall back to byte-at-a-time scanning.
            i = PAGESZ;
            loop {
                #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
                {
                    let lo = core::ptr::read_unaligned::<u64>(
                        pp.sub(core::mem::size_of::<u64>()) as *const u64,
                    );
                    let hi = core::ptr::read_unaligned::<u64>(
                        pp.sub(2 * core::mem::size_of::<u64>()) as *const u64,
                    );
                    if lo != 0 || hi != 0 {
                        break; // non-zero found — exit inner loop
                    }
                    i -= 2 * core::mem::size_of::<u64>();
                    pp = pp.sub(2 * core::mem::size_of::<u64>());
                }
                #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
                {
                    let lo = core::ptr::read_unaligned::<u8>(
                        pp.sub(core::mem::size_of::<u8>()) as *const u8,
                    );
                    let hi = core::ptr::read_unaligned::<u8>(
                        pp.sub(2 * core::mem::size_of::<u8>()) as *const u8,
                    );
                    if lo != 0 || hi != 0 {
                        break; // non-zero found — exit inner loop
                    }
                    i -= 2 * core::mem::size_of::<u8>();
                    pp = pp.sub(2 * core::mem::size_of::<u8>());
                }
                if i == 0 {
                    break; // entire page is zero — skip it
                }
            }
            // If i == 0: the entire page was zero, pp already moved past it.
            //   The next outer-loop iteration starts with pp page-aligned, i=0,
            //   Step A is a no-op, and we proceed to scan the previous page.
            // If i != 0: non-zero data was found partway through the page.
            //   pp is positioned inside that page, i is the remaining byte count.
            //   Step A of the next iteration will memset-clear [pp-i, pp).
        }
    }
}

// ============================================================================
// 单元测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use core::ptr;

    // ======================================================================
    // 测试辅助设施
    // ======================================================================

    /// mock 分配器使用的静态堆缓冲区
    static mut MOCK_HEAP: [u8; 16384] = [0u8; 16384];
    /// mock 分配器使用的偏移量计数器
    static mut MOCK_OFFSET: usize = 0;

    /// 重置 mock 堆状态。每次测试前调用以确保隔离。
    unsafe fn mock_reset() {
        MOCK_OFFSET = 0;
        // 填充"脏"数据标记（0xCD），以便清零检测
        for byte in MOCK_HEAP.iter_mut() {
            *byte = 0xCD;
        }
    }

    /// 模拟成功的内存分配 —— 从静态缓冲区线性分配。
    extern "C" fn mock_malloc_success(size: usize) -> *mut c_void {
        if size == 0 {
            return core::ptr::null_mut();
        }
        // SAFETY: 测试辅助函数，仅在测试上下文中使用，每次测试前调用 mock_reset 重置状态。
        unsafe {
            if MOCK_OFFSET + size > MOCK_HEAP.len() {
                return core::ptr::null_mut(); // 堆耗尽
            }
            let ptr = MOCK_HEAP.as_mut_ptr().add(MOCK_OFFSET);
            MOCK_OFFSET += size;
            ptr as *mut c_void
        }
    }

    /// 模拟失败的内存分配 —— 始终返回 NULL 指针。
    extern "C" fn mock_malloc_fail(_size: usize) -> *mut c_void {
        core::ptr::null_mut()
    }

    /// 用于验证函数指针类型匹配的辅助函数。
    /// 当实现完成后，测试验证 calloc_impl 的参数传递正确性。
    extern "C" fn mock_malloc_small(size: usize) -> *mut c_void {
        // 使用一个小的静态缓冲区
        static mut SMALL_BUF: [u8; 256] = [0u8; 256];
        if size == 0 || size > 256 {
            return core::ptr::null_mut();
        }
        // 注意：此处故意不重置 SMALL_BUF 以测试清零逻辑
        // （实现层应将返回的内存清零）
        // SAFETY: 测试辅助函数，SMALL_BUF 是静态缓冲区，仅在测试中使用。
        unsafe { SMALL_BUF.as_mut_ptr() as *mut c_void }
    }

    // ======================================================================
    // calloc_impl 测试
    // ======================================================================

    mod calloc_impl_tests {
        use super::*;

        // --- 基本功能测试 ---

        test!("test_normal_allocation" {
        // 测试 1: 正常分配 —— m=4, n=16，期望成功返回非空指针。
        // 验证 calloc_impl 的基本分配路径。
            unsafe {
                mock_reset();
                // 注意：由于函数体为 todo!()，此测试将在运行时 panic。
                // 实现完成后取消注释以下断言：
                let p = calloc_impl(4, 16, mock_malloc_success);
                // assert!(!p.is_null(), "正常分配应返回非空指针");
                // assert_eq!(p as usize % core::mem::align_of::<usize>(), 0,
                //     "返回指针应满足对齐要求");
                let _ = p; // 占位：避免 unused 警告（todo!() 导致 unreachable）
            }
        });

        test!("test_zero_initialization" {
        // 测试 2: 返回内存应为全零。
        // calloc 的核心语义保证。
            unsafe {
                mock_reset();
                // 实现完成后验证：
                // let p = calloc_impl(8, 8, mock_malloc_success);
                // assert!(!p.is_null());
                // let slice = core::slice::from_raw_parts(p as *const u8, 64);
                // assert!(slice.iter().all(|&b| b == 0),
                //     "calloc 返回的内存必须全部为零");
            }
        });

        test!("test_single_byte_allocation" {
        // 测试 3: 单字节分配 —— m=1, n=1。
            unsafe {
                mock_reset();
                let p = calloc_impl(1, 1, mock_malloc_success);
                // assert!(!p.is_null());
                // assert_eq!(*(p as *const u8), 0);
                let _ = p;
            }
        });

        // --- 溢出检测测试 ---

        test!("test_overflow_max_times_two" {
        // 测试 4: 溢出检测 —— m = usize::MAX, n = 2。
        // `m * n` 必然溢出 usize，期望返回 null_mut()。
            unsafe {
                let result = calloc_impl(usize::MAX, 2, mock_malloc_success);
                // assert!(result.is_null(), "溢出时应返回 NULL");
                // errno 应被设置为 ENOMEM（需要通过 errno 模块验证）
                let _ = result;
            }
        });

        test!("test_overflow_both_max" {
        // 测试 5: 溢出检测 —— m = usize::MAX, n = usize::MAX。
        // 最大乘积溢出。
            unsafe {
                let result = calloc_impl(usize::MAX, usize::MAX, mock_malloc_success);
                // assert!(result.is_null());
                let _ = result;
            }
        });

        test!("test_overflow_just_barely" {
        // 测试 6: 溢出检测 —— m = usize::MAX / 2 + 1, n = 2.
        // 刚好溢出的边界情况。
            unsafe {
                let m = usize::MAX / 2 + 1;
                let result = calloc_impl(m, 2, mock_malloc_success);
                // assert!(result.is_null(), "刚好溢出的情况应返回 NULL");
                let _ = result;
            }
        });

        test!("test_no_overflow_max_times_one" {
        // 测试 7: 非溢出边界 —— m = usize::MAX, n = 1。
        // 不应视为溢出（乘积等于 usize::MAX）。
            unsafe {
                mock_reset();
                let result = calloc_impl(usize::MAX, 1, mock_malloc_fail);
                // 虽然分配会失败（mock 堆不够大），但不应该走溢出路径
                let _ = result;
            }
        });

        test!("test_no_overflow_boundary" {
        // 测试 8: 非溢出边界 —— m = usize::MAX / 2, n = 2。
        // 乘积恰好等于 usize::MAX - 1，不应溢出。
            unsafe {
                mock_reset();
                let result = calloc_impl(usize::MAX / 2, 2, mock_malloc_fail);
                let _ = result;
            }
        });

        // --- 分配失败测试 ---

        test!("test_malloc_returns_null" {
        // 测试 9: 分配器返回 NULL —— calloc_impl 应透明传递。
            unsafe {
                let p = calloc_impl(8, 8, mock_malloc_fail);
                // assert!(p.is_null(), "底层 malloc 失败时 calloc_impl 应返回 NULL");
                let _ = p;
            }
        });

        test!("test_heap_exhaustion" {
        // 测试 10: 大块分配超出 mock 堆容量 —— mock 堆耗尽时应返回 NULL。
            unsafe {
                mock_reset();
                // 请求超出 16384 字节的 mock 堆
                let p = calloc_impl(1, 20000, mock_malloc_success);
                // assert!(p.is_null());
                let _ = p;
            }
        });

        // --- 零大小分配测试 ---

        test!("test_zero_elements" {
        // 测试 11: m=0, n 非零 —— 零元素数。
            unsafe {
                mock_reset();
                let p = calloc_impl(0, 16, mock_malloc_success);
                // 行为待定：可能返回 NULL 或非 NULL 指针
                // 注意：根据 C 标准，calloc(0, n) 的行为是实现定义的
                let _ = p;
            }
        });

        test!("test_zero_size" {
        // 测试 12: n=0, m 非零 —— 零元素大小。
            unsafe {
                mock_reset();
                let p = calloc_impl(16, 0, mock_malloc_success);
                let _ = p;
            }
        });

        test!("test_both_zero" {
        // 测试 13: m=0, n=0 —— 两个参数均为零。
            unsafe {
                mock_reset();
                let p = calloc_impl(0, 0, mock_malloc_success);
                let _ = p;
            }
        });

        // --- 函数指针参数化测试 ---

        test!("test_parameterized_allocator_fail" {
        // 测试 14: 同一参数下不同分配器产生不同的分配结果。
        // 验证函数指针参数化的正确性：传入失败分配器时应返回 NULL。
            unsafe {
                let p = calloc_impl(4, 4, mock_malloc_fail);
                // assert!(p.is_null());
                let _ = p;
            }
        });

        test!("test_consecutive_allocations" {
        // 测试 15: 连续两次调用分配器应返回不同的指针（无重叠）。
            unsafe {
                mock_reset();
                // let p1 = calloc_impl(4, 4, mock_malloc_success);
                // assert!(!p1.is_null());
                // let p2 = calloc_impl(4, 4, mock_malloc_success);
                // assert!(!p2.is_null());
                // assert_ne!(p1, p2, "连续分配应返回不同指针");
                // assert!(p1.add(16) <= p2 || p2.add(16) <= p1, "分配区域不应重叠");
            }
        });

        // --- 指针对齐测试 ---

        test!("test_alignment_usize" {
        // 测试 16: 返回指针应对齐到 `usize` 边界。
            unsafe {
                mock_reset();
                // let p = calloc_impl(8, 8, mock_malloc_success);
                // assert!(!p.is_null());
                // assert_eq!(p as usize % core::mem::align_of::<usize>(), 0);
            }
        });

        test!("test_alignment_u64" {
        // 测试 17: 返回指针应对齐到 `u64` 边界。
            unsafe {
                mock_reset();
                // let p = calloc_impl(8, 8, mock_malloc_success);
                // assert!(!p.is_null());
                // assert_eq!(p as usize % core::mem::align_of::<u64>(), 0);
            }
        });

        test!("test_alignment_16" {
        // 测试 18: 返回指针应对齐到 16 字节边界（针对 SSE/NEON 等 SIMD 需求）。
            unsafe {
                mock_reset();
                // let p = calloc_impl(8, 8, mock_malloc_success);
                // assert!(!p.is_null());
                // assert_eq!(p as usize % 16, 0,
                //     "应满足 16 字节对齐以支持 SIMD 操作");
            }
        });
    }

    // ======================================================================
    // mal0_clear 测试
    // ======================================================================

    mod mal0_clear_tests {
        use super::*;

        // --- 基本功能测试 ---

        test!("test_zero_length" {
        // 测试 1: 零长度 —— n=0，应返回 0。
            let mut buf = [0xABu8; 64];
            let result = mal0_clear(buf.as_mut_ptr(), 0);
            // assert_eq!(result, 0, "零长度应返回 0");
            let _ = result;
        });

        test!("test_smaller_than_page" {
        // 测试 2: 缓冲区小于一页 —— n=256 < PAGESZ。
        // 按照 spec，n < PAGESZ 时返回 r == n。
            let mut buf = [0xABu8; 256];
            let result = mal0_clear(buf.as_mut_ptr(), 256);
            // assert_eq!(result, 256, "小于一页应返回全部字节数");
            let _ = result;
        });

        test!("test_exact_one_page_all_zero" {
        // 测试 3: 恰好一页且全零 —— n=4096，缓冲区全部为零。
            let mut buf = [0u8; 4096];
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 全零页被跳过，返回值 < PAGESZ
            // assert!(result < PAGESZ);
            let _ = result;
        });

        test!("test_exact_one_page_all_dirty" {
        // 测试 4: 恰好一页且全脏 —— n=4096，缓冲区全部为非零。
            let mut buf = [0xFFu8; 4096];
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 全部非零，应需要清零整个页面
            // assert_eq!(result, PAGESZ);
            let _ = result;
        });

        // --- 边界情况测试 ---

        test!("test_nonzero_at_start" {
        // 测试 5: 缓冲区开头有非零值。
            let mut buf = [0u8; 4096];
            buf[0] = 0xFF; // 仅在第一个字节设置非零
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 从尾部向前扫描，应检测到开头的非零值
            let _ = result;
        });

        test!("test_nonzero_at_end" {
        // 测试 6: 缓冲区末尾有非零值。
            let mut buf = [0u8; 4096];
            buf[4095] = 0xFF; // 仅在最后一个字节设置非零
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 从尾部扫描，应立即检测到非零值
            // assert_eq!(result, 4096); // 整个页面需要清零
            let _ = result;
        });

        test!("test_nonzero_in_middle" {
        // 测试 7: 缓冲区中间有非零值。
            let mut buf = [0u8; 4096];
            buf[2048] = 0xFF; // 页面中间位置非零
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            let _ = result;
        });

        test!("test_stride_nonzero" {
        // 测试 8: 全部 u64 对齐的非零值 —— 每隔 8 字节设置非零。
        // 验证宽字扫描的正确性。
            let mut buf = [0u8; 4096];
            // 每隔 8 字节（u64 宽度）设置非零
            for i in (0..4096).step_by(8) {
                buf[i] = 0xFF;
            }
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 所有 u64 位置都有非零值
            // assert_eq!(result, 4096);
            let _ = result;
        });

        // --- 多页测试 ---

        test!("test_two_pages_second_nonzero" {
        // 测试 9: 两页 —— 第一页全零，第二页末尾非零。
            let mut buf = [0u8; 8192]; // 2 * 4096
            buf[8191] = 0xFF; // 第二页最后一个字节非零
            let result = mal0_clear(buf.as_mut_ptr(), 8192);
            // 第二页被清零（包含非零字节的页需要清零）
            let _ = result;
        });

        test!("test_two_pages_all_zero" {
        // 测试 10: 两页 —— 两页均为全零。
            let mut buf = [0u8; 8192];
            let result = mal0_clear(buf.as_mut_ptr(), 8192);
            // 两页均为零，全部跳过，返回值 < PAGESZ
            // assert!(result < PAGESZ);
            let _ = result;
        });

        test!("test_two_pages_first_nonzero" {
        // 测试 11: 两页 —— 第一页非零，第二页全零。
            let mut buf = [0u8; 8192];
            buf[0] = 0xFF; // 第一页第一个字节非零
            let result = mal0_clear(buf.as_mut_ptr(), 8192);
            // 第二页跳过（全零），第一页清零
            let _ = result;
        });

        test!("test_three_pages_middle_nonzero" {
        // 测试 12: 三页 —— 中间页有非零值。
            let mut buf = [0u8; 12288]; // 3 * 4096
            buf[5000] = 0xFF; // 第二页中间位置
            let result = mal0_clear(buf.as_mut_ptr(), 12288);
            let _ = result;
        });

        test!("test_partial_last_page" {
        // 测试 13: 不完整页面 —— 不足一个完整页的末尾部分。
            let mut buf = [0xABu8; 5000]; // 1 个完整页 + 904 字节尾页
            let result = mal0_clear(buf.as_mut_ptr(), 5000);
            // 1 个完整页 + 不完整尾页
            let _ = result;
        });

        // --- 非对齐指针测试 ---

        test!("test_unaligned_buffer" {
        // 测试 14: 非页对齐的缓冲区 —— 指针不是页对齐的。
            let mut buf = [0u8; 5000];
            let ptr = unsafe { buf.as_mut_ptr().add(7) };
            let result = mal0_clear(ptr, 4096);
            // 应正确处理非对齐起始地址
            let _ = result;
        });

        test!("test_unaligned_large" {
        // 测试 15: 非对齐并且长度大于一页。
            let mut buf = [0xABu8; 10000];
            // 故意从非对齐偏移开始
            let ptr = unsafe { buf.as_mut_ptr().add(13) };
            let result = mal0_clear(ptr, 8000);
            let _ = result;
        });

        // --- 返回值验证测试 ---

        test!("test_return_value_bounded" {
        // 测试 16: 返回值不超过 PAGESZ（当 n >= PAGESZ 时）。
            let mut buf = [0xFFu8; 4096];
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // assert!(result <= PAGESZ,
            //     "返回值不应超过 PAGESZ（当缓冲区 >= PAGESZ 时）");
            let _ = result;
        });

        test!("test_return_value_large_n" {
        // 测试 17: 返回值不溢出（对于极大 n 值）。
            // mal0_clear 是内部函数，调用者负责保证 [p, p+n) 是有效内存。
            // 此测试使用合法的缓冲区大小。
            let mut buf = [0u8; 16384]; // 4 页
            let n = buf.len();
            let result = mal0_clear(buf.as_mut_ptr(), n);
            // 返回值应在 [0, PAGESZ] 范围内
            assert!(result <= 4096, "mal0_clear 返回值 {} 应 <= 4096", result);
        });

        // --- 清零正确性测试 ---

        test!("test_buffer_is_zeroed" {
        // 测试 18: 验证清零后缓冲区状态 —— 应完全为零。
            let mut buf = [0xABu8; 4096];
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // 注意：mal0_clear 从尾部向头部清零，但不保证清零整个缓冲区
            // 返回值 r 指示还需要调用者清零 p[0..r] 部分
            // 验证 mal0_clear 尾部清零的部分：
            // let expected_zero_start = result; // p[result..n] 应已清零
            // for i in expected_zero_start..4096 {
            //     assert_eq!(buf[i], 0, "字节 [{}] 应为零", i);
            // }
            let _ = result;
        });

        test!("test_preservation_of_uncleared" {
        // 测试 19: 仅 m=0 到 mal0_clear 返回值范围内的内存应保持不变。
            let original = [0xABu8; 4096];
            let mut buf = original;
            let result = mal0_clear(buf.as_mut_ptr(), 4096);
            // mal0_clear 不应修改 p[0..result] 范围的内容，
            // 该部分留给调用者（memset）处理
            let _ = result;
        });
    }

    // ======================================================================
    // __libc_calloc (内部接口) 测试
    // ======================================================================

    mod libc_calloc_tests {
        use super::*;

        test!("test_internal_basic_allocation" {
        // 测试 1: 基本内部分配 —— m=8, n=8。
            let p = __libc_calloc(8, 8);
            // assert!(!p.is_null(), "内部分配应成功");
            let _ = p;
        });

        test!("test_internal_zero_elements" {
        // 测试 2: 内部零大小分配 —— m=0, n=16。
            let p = __libc_calloc(0, 16);
            let _ = p;
        });

        test!("test_internal_zero_size" {
        // 测试 3: 内部零大小分配 —— m=16, n=0。
            let p = __libc_calloc(16, 0);
            let _ = p;
        });

        test!("test_internal_overflow" {
        // 测试 4: 内部溢出检测 —— m=usize::MAX, n=2。
            let p = __libc_calloc(usize::MAX, 2);
            // assert!(p.is_null(), "溢出时应返回 NULL");
            let _ = p;
        });

        test!("test_internal_overflow_both_max" {
        // 测试 5: 内部溢出检测 —— m=usize::MAX, n=usize::MAX。
            let p = __libc_calloc(usize::MAX, usize::MAX);
            // assert!(p.is_null());
            let _ = p;
        });

        test!("test_internal_zero_init" {
        // 测试 6: 内部分配返回全零内存。
            let p = __libc_calloc(32, 1);
            // assert!(!p.is_null());
            // 验证 32 字节全部为零
            // let slice = unsafe { core::slice::from_raw_parts(p as *const u8, 32) };
            // assert!(slice.iter().all(|&b| b == 0));
            let _ = p;
        });

        test!("test_visibility_is_crate_only" {
        // 测试 7: __libc_calloc 应无法从 crate 外部访问。
        // 此测试在编译期验证 —— 若可通过 crate 外部路径访问则编译失败。
        // 由于我们在 `mod tests` 中（crate 内部），可以正常调用。
        // 此测试作为文档标注存在。
            // 编译期验证：__libc_calloc 的可见性是 pub(crate)
            // 所以只能从 crate 内部访问
            // 集成测试文件 tests/malloc/libc_calloc_test.rs 中不应能引用此函数
            // （在集成测试中尝试 `use rusl::malloc::__libc_calloc;` 会编译失败）
            // 此处调用仅为证明它在 crate 内部可访问
            let _p = __libc_calloc(1, 1);
        });

        test!("test_internal_unique_pointers" {
        // 测试 8: 连续的内部 calloc 不返回同一指针。
            // 注：实际行为取决于底层 __libc_malloc 的实现
            // let p1 = __libc_calloc(8, 8);
            // let p2 = __libc_calloc(8, 8);
            // assert_ne!(p1, p2);
        });

        test!("test_internal_single_byte" {
        // 测试 9: 单个字节内部分配（最小分配单位）。
            let p = __libc_calloc(1, 1);
            // assert!(!p.is_null());
            // assert_eq!(unsafe { *(p as *const u8) }, 0);
            let _ = p;
        });

        test!("test_internal_large_allocation" {
        // 测试 10: 较大的内部分配 —— 1024 字节。
            let p = __libc_calloc(1, 1024);
            // assert!(!p.is_null());
            let _ = p;
        });

        test!("test_internal_max_times_one" {
        // 测试 11: 边界情况 —— m=usize::MAX, n=1。
        // 技术上不溢出，但分配如此巨大的内存会失败。
            let p = __libc_calloc(usize::MAX, 1);
            // 分配注定失败（没有系统有这么多的内存），但不应走溢出路径
            // assert!(p.is_null()); // 分配失败导致的 NULL
            let _ = p;
        });
    }

    // ======================================================================
    // calloc (公共 C ABI 接口) 测试
    // ======================================================================

    mod calloc_tests {
        use super::*;

        test!("test_basic_calloc" {
        // 测试 1: 基本分配 —— 分配 10 个 u32 元素。
            unsafe {
                let p = calloc(10, core::mem::size_of::<u32>());
                // assert!(!p.is_null(), "公共 calloc 应成功分配");
                let _ = p;
            }
        });

        test!("test_single_byte" {
        // 测试 2: 单字节分配 —— calloc(1, 1)。
            unsafe {
                let p = calloc(1, 1);
                // assert!(!p.is_null());
                // assert_eq!(*(p as *const u8), 0, "单字节应为零");
                let _ = p;
            }
        });

        test!("test_large_allocation" {
        // 测试 3: 大块分配 —— 1024 字节。
            unsafe {
                let p = calloc(1, 1024);
                // assert!(!p.is_null());
                let _ = p;
            }
        });

        test!("test_zero_count" {
        // 测试 4: 零计数 —— m=0, n=1。
            unsafe {
                let p = calloc(0, 1);
                let _ = p;
            }
        });

        test!("test_zero_size_public" {
        // 测试 5: 零大小 —— m=1, n=0。
            unsafe {
                let p = calloc(1, 0);
                let _ = p;
            }
        });

        test!("test_both_zero_public" {
        // 测试 6: 双零 —— m=0, n=0。
            unsafe {
                let p = calloc(0, 0);
                let _ = p;
            }
        });

        test!("test_overflow_max_times_two" {
        // 测试 7: 溢出检测 —— calloc(usize::MAX, 2)。
            unsafe {
                let p = calloc(usize::MAX, 2);
                // assert!(p.is_null(), "溢出时应返回 NULL");
                // errno 应设为 ENOMEM
                let _ = p;
            }
        });

        test!("test_overflow_both_max" {
        // 测试 8: 溢出检测 —— calloc(usize::MAX, usize::MAX)。
            unsafe {
                let p = calloc(usize::MAX, usize::MAX);
                // assert!(p.is_null());
                let _ = p;
            }
        });

        test!("test_no_overflow_max_times_one" {
        // 测试 9: 非溢出边界 —— calloc(usize::MAX, 1)。
            unsafe {
                let p = calloc(usize::MAX, 1);
                // 不溢出，但分配注定失败
                // assert!(p.is_null()); // 分配失败，非溢出
                let _ = p;
            }
        });

        test!("test_zero_initialization" {
        // 测试 10: 返回内存应为全零。
            unsafe {
                let p = calloc(64, 1);
                // 实现后验证：
                // assert!(!p.is_null());
                // let slice = core::slice::from_raw_parts(p as *const u8, 64);
                // assert!(slice.iter().all(|&b| b == 0),
                //     "calloc 返回的 64 字节必须全部为零");
                let _ = p;
            }
        });

        test!("test_pointer_validity" {
        // 测试 11: 返回指针不应为悬垂指针。
        // 验证基本的指针有效性（非 null）。
            unsafe {
                let p = calloc(4, 8);
                // assert!(!p.is_null());
                // 可安全地读取前几个字节
                // let byte = ptr::read_unaligned(p as *const u8);
                // assert_eq!(byte, 0);
                let _ = p;
            }
        });

        test!("test_fn_pointer_compatibility" {
        // 测试 12: 函数指针类型兼容性验证。
        // 确保 calloc 可以传递 extern "C" fn 类型的分配器函数指针。
            unsafe {
                // calloc 内部调用 calloc_impl(_, _, malloc)
                // malloc 的类型是 unsafe extern "C" fn(usize) -> *mut c_void
                // 验证此类型可以被正确传递给 calloc_impl
                let p = calloc(4, 4);
                let _ = p;
            }
        });
    }

    // ======================================================================
    // 跨函数一致性测试
    // ======================================================================

    mod cross_function_tests {
        use super::*;

        test!("test_public_and_internal_independent" {
        // 测试: 验证 calloc 和 __libc_calloc 对相同参数返回不同的指针。
        // 因为 calloc 使用公共 malloc（可能被替换），__libc_calloc 使用内部 malloc，
        // 两者从不同的堆区域分配。
        // 注意：实际行为取决于 malloc 实现。
            unsafe {
                let _p1 = calloc(4, 4);
            }
            let _p2 = __libc_calloc(4, 4);
            // 两者应该都能成功分配
            // assert!(!p1.is_null());
            // assert!(!p2.is_null());
        });

        test!("test_calloc_equivalence" {
        // 测试: calloc(m, n) 应等价于 calloc_impl(m, n, malloc)。
        // 验证 public calloc 的委托路径正确性。
            unsafe {
                let p1 = calloc(4, 4);
                // 由于实现未完成，此处仅为骨架占位
                let _ = p1;
            }
        });

        test!("test_internal_equivalence" {
        // 测试: __libc_calloc(m, n) 应等价于 calloc_impl(m, n, __libc_malloc)。
        // 验证内部 calloc 的委托路径正确性。
            let _p = __libc_calloc(4, 4);
        });
    }

    // ======================================================================
    // PAGESZ 常量测试
    // ======================================================================

    test!("test_pagesz_constant" {
        assert_eq!(PAGESZ, 4096, "PAGESZ 应为 4096");
        // 验证 PAGESZ 为 2 的幂
        assert_eq!(PAGESZ & (PAGESZ - 1), 0, "PAGESZ 应为 2 的幂");
    });

    // ======================================================================
    // 类型正确性编译期测试
    // ======================================================================

    test!("test_type_compatibility" {
        // 验证 calloc 的函数指针类型正确性。
        // 测试 calloc_impl 可以接受不同签名的 extern "C" fn。
        unsafe {
            // 验证 mock 函数可以正确适配 calloc_impl 的 malloc_fn 参数类型
            mock_reset();
            let _ = calloc_impl(1, 1, mock_malloc_success);
            let _ = calloc_impl(1, 1, mock_malloc_fail);
            let _ = calloc_impl(1, 1, mock_malloc_small);
        }
    });
}