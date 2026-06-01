// free.rs — POSIX free() 函数 (薄封装)
//
// 对应 C 源文件: src/malloc/free.rs (musl)
// 声明: <stdlib.h>
//
// 本模块仅定义一层薄封装，将 POSIX `free(void *p)` 转发给内部实现
// `__libc_free`。实际分配器算法位于 `src/malloc/mallocng/` 模块。
//
// 对外导出符号:
//   free  — C 标准函数，释放动态分配的内存

use core::ffi::c_void;

/// 释放先前由 malloc/calloc/realloc/aligned_alloc 返回的内存块。
///
/// # 行为
///
/// - **p.is_null()**: 函数立即返回，无任何操作。这是 C 标准要求的无操作行为。
/// - **p 非空**: 指针指向的内存被标记为可供后续分配重用。释放后 `p` 自身的值不变，
///   但变为悬垂指针，再次解引用或释放均为未定义行为。
///
/// # Safety
///
/// 调用者必须确保:
/// - `p` 必须是之前由 `malloc`、`calloc`、`realloc`、`aligned_alloc` 或
///   `posix_memalign` 返回的有效指针，**或**为 `NULL`
/// - 若 `p` 非空，其指向的内存必须尚未被释放（double-free 导致未定义行为；
///   rusl 通过断言和头部失效化提供 best-effort 检测）
/// - 调用者不持有任何 malloc 相关的内部锁（本函数内部自行处理同步）
///
/// # 错误处理
///
/// 无返回值，不设置 `errno`（C 标准规定 `free()` 不报告错误）。
/// 内部的 `sys_madvise` 和 `sys_munmap` 系统调用可能修改 errno，但本函数
/// 保证调用者的 `errno` 值不被改变（保存/恢复机制）。
///
/// # 线程安全
///
/// 完全线程安全。通过内部锁 (`wrlock`/`unlock`) 保护全局分配器状态，
/// 并在 fast-path 路径上使用 `AtomicI32::compare_exchange` 无锁 CAS
/// 原子操作优化高并发场景。
///
/// # 信号安全
///
/// **不是** async-signal-safe。持有锁期间被信号中断可能导致死锁。
///
/// # 实现架构
///
/// 采用分层策略:
/// 1. **Fast-path (无锁原子释放)**: 同组内非首/非末释放，直接原子 CAS 更新
///    `freed_mask`
/// 2. **Slow-path (加锁释放)**: 首/末释放触发整组回收、mmap 解除映射等
/// 3. **Page-level reclamation**: 大槽位中完整空闲物理页通过
///    `sys_madvise(MADV_FREE)` 向内核提示可回收 (当前默认禁用)
#[no_mangle]
pub unsafe extern "C" fn free(p: *mut c_void) {
    // 直接委托给 mallocng 内部实现。
    // C 标准要求: free(NULL) 为无操作，此检查由 __libc_free 内部完成。
    super::mallocng::free::__libc_free(p);
}