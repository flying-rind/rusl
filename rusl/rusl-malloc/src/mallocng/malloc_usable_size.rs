//! malloc_usable_size — 获取已分配内存块的实际可用大小 (GNU 扩展)
//! [Visibility]: Public, <malloc.h>
//!
//! 对应 musl 的 `src/malloc/mallocng/malloc_usable_size.c`。
//!
//! ## 算法概述
//!
//! `malloc_usable_size` 是完全的纯读操作，通过 in-band header 中的自描述信息
//! 在 O(1) 时间内计算出可用大小，无需遍历任何全局数据结构:
//!
//! 1. **NULL 处理**: 若 `p.is_null()`，返回 `0` (GNU 扩展约定)
//! 2. **逆推 Meta 控制块** (`get_meta`): 从 `p` 的 in-band header 读取偏移量，
//!    逆推 Group 基址，执行多层 assert 校验获取所属 Meta
//! 3. **获取 slot 跨步** (`get_stride`): 统一常规 slab 和 mmap 两种模式
//! 4. **定位 slot 边界**: `start = Group::storage + stride * idx`,
//!    `end = start + stride - IB`
//! 5. **解码可用大小** (`get_nominal_size`): 从 in-band header 解码
//!    reserved 字段，计算实际可用字节数
//!
//! ## 依赖模块 (均为 crate-internal)
//!
//! - `super::meta` — Meta, Group, UNIT, IB, SIZE_CLASSES, get_meta, get_slot_index,
//!   get_stride, get_nominal_size
//! - `super::glue` — PGSZ, assert_or_crash (间接依赖，经 get_meta 的 assert 链引入)
//!
//! ## 线程安全性
//!
//! 本函数为无锁设计 (lock-free):
//! - 不调用 `wrlock()` / `unlock()`
//! - 对 `avail_mask` / `freed_mask` 仅执行原子加载 (AtomicI32::load)
//! - 不修改任何全局状态
//! - 但若另一线程并发 `free()` 或 `realloc()` 同一指针，行为未定义 (use-after-free)
//!
//! ## 与 GNU/POSIX 标准的关系
//!
//! `malloc_usable_size` 是 GNU 扩展 (`<malloc.h>` 声明)，POSIX 标准未定义。
//! 可移植代码应避免依赖此函数。

use core::ffi::c_void;

// ---------------------------------------------------------------------------
// 对外导出接口
// ---------------------------------------------------------------------------

/// 返回 `p` 所指向内存块的实际可用字节数。
///
/// 通过读取分配块的 in-band header 中的自描述元数据，在 O(1) 时间内计算出
/// 用户可安全使用的字节数上限。
///
/// # 参数
///
/// - `p`: 由 `malloc()`、`calloc()` 或 `realloc()` 返回的有效指针，
///        或 `core::ptr::null_mut()` (此时返回 0)。
///
/// # 返回值
///
/// - 若 `p == NULL`，则返回 `0` (GNU 扩展行为约定)。
/// - 若 `p != NULL` 且为有效分配指针，则返回实际可用字节数，
///   该值总是不小于原始请求大小（大小类别取整可能导致实际分配更大）。
///   上界为所在 slot 的 `stride - IB - reserved`。
///
/// # Safety
///
/// 调用者必须确保以下前置条件全部成立，否则行为未定义:
///
/// - **指针来源**: `p` 若不为 NULL，必须是由本分配器 (rusl mallocng) 的
///   `malloc()`、`calloc()` 或 `realloc()` 返回的有效指针，且尚未经 `free()` 释放。
/// - **对齐约束**: `p` 若不为 NULL，必须满足 `(p as usize) & 15 == 0`
///   (16 字节对齐)。违反此条件将触发 `get_meta` 内部的 assert → abort()。
/// - **并发约束**: 不得在另一线程对同一指针执行 `free()` 或 `realloc()` 期间
///   调用本函数。违反此条件触发 use-after-free 未定义行为。
///
/// # 不变量
///
/// - **INV-SIZE-LOWER-BOUND**: `malloc_usable_size(malloc(n)) >= n`
/// - **INV-CALLOC-BOUND**: `malloc_usable_size(calloc(nmemb, size)) >= nmemb * size`
/// - **INV-REALLOC-BOUND**: `malloc_usable_size(realloc(p, n)) >= n`
/// - **INV-NO-LOCK**: 本函数不获取也不释放任何锁
/// - **INV-READ-ONLY**: 本函数不修改任何全局状态 (avail_mask / freed_mask / CTX 均只读)
///
/// # 实现算法 (O(1) 时间)
///
/// ```ignore
/// pub extern "C" fn malloc_usable_size(p: *mut c_void) -> usize {
///     if p.is_null() {
///         return 0;
///     }
///     let p = p as *const u8;
///     let meta = get_meta(p);                         // 13 步 assert 校验链
///     let idx = get_slot_index(p);                     // p[-3] & 31
///     let stride = get_stride(meta);                   // UNIT * SIZE_CLASSES[sc] 或 maplen*PGSZ-UNIT
///     let start = (meta.mem as *const u8).offset(UNIT) // Group::storage 起始
///                     .offset(stride * idx);           // 定位到具体 slot
///     let end = start.offset(stride - IB);             // slot 末尾减去 in-band 元数据
///     get_nominal_size(p, end)                         // 解码用户可用字节数
/// }
/// ```
///
/// # 注意事项
///
/// - 返回值不可用于推断原始请求大小 —— 仅表示分配器实际预留空间
/// - 多线程并发 free/realloc 同一指针会导致 use-after-free，行为未定义
/// - get_meta 内部的 assert 在 Release 构建中是否移除取决于 rusl 的构建配置
///   (建议保留核心校验以保证堆损坏检测能力)
#[no_mangle]
#[allow(unused_variables)]
pub extern "C" fn malloc_usable_size(p: *mut c_void) -> usize {
    // 1) NULL 处理: GNU 扩展约定, malloc_usable_size(NULL) == 0
    if p.is_null() {
        return 0;
    }

    // SAFETY: p 非 NULL 时，调用者保证 p 是由本分配器返回的有效指针且未被释放。
    // 后续所有操作（get_meta, get_slot_index, get_stride, get_nominal_size）均为
    // 读取分配器内部元数据的 unsafe 操作。
    unsafe {
        let p = p as *const u8;

        // 2) 逆推 Meta 控制块 (13 步 assert 校验链)
        let g = super::meta::get_meta(p);

        // 3) 获取槽位索引: p[-3] & 31
        let idx = super::meta::get_slot_index(p);

        // 4) 获取槽位跨步大小
        //    Case 1 (mmap): stride = maplen * PGSZ - UNIT
        //    Case 2 (slab):  stride = UNIT * SIZE_CLASSES[sc]
        let stride = super::meta::get_stride(g);

        // 5) 定位 slot 边界
        //    storage[] 柔性数组紧接在 Group header 之后 (偏移 UNIT 字节)
        let start = ((*g).mem as *const u8)
            .add(super::meta::UNIT)
            .add(stride * idx);
        //    slot 末尾减去 in-band 元数据 (IB = 4 字节)
        let end = start.add(stride - super::meta::IB);

        // 6) 从 in-band header 解码用户可用字节数
        super::meta::get_nominal_size(p, end)
    }
}