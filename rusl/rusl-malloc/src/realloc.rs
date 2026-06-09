//! `realloc` 函数 —— POSIX 标准内存重新分配。
//!
//! 本模块是 rusl 对外导出的 `realloc` 公共入口点。函数使用 `extern "C"` ABI，
//! 与 C 标准 `<stdlib.h>` 中声明的 `void *realloc(void *ptr, size_t size)` 保持
//! ABI 兼容。
//!
//! 实际内存管理逻辑位于 `mallocng` 子模块，通过 `realloc_impl` 实现。

use core::ffi::c_void;

/// 更改 `p` 指向的内存块大小为 `n` 字节。
///
/// 此函数是 POSIX.1-2001 / ISO C89 标准中 `realloc` 的 rusl 实现。
/// 实际逻辑委托给内部函数 `realloc_impl`。
///
/// # 行为规则
///
/// | `p` | `n` | 行为 |
/// |-----|-----|------|
/// | `NULL` | `>0` | 等价于 `malloc(n)` |
/// | `非 NULL` | `0` | 等价于 `free(p)`，返回 `NULL` |
/// | `非 NULL` | `>0` 且 `≤` 原大小 | 原地缩容或保持，可能返回原指针 |
/// | `非 NULL` | `>0` 且 `>` 原大小 | 尝试原地扩容，否则分配新块拷贝后释放旧块 |
///
/// # 返回值说明
///
/// - **成功**: 返回指向新内存块的指针，该指针对齐到适合任何对象类型的边界
///   - 若原地调整或 mremap 成功，返回的指针等于原 `p`
///   - 若需要移动，返回新指针，旧块已被释放
///   - 若 `n > 旧大小`，超出部分的**内容未初始化**
/// - **失败**: 返回 `NULL`，设置 `errno = ENOMEM`，原内存块 `p` 保持有效且内容不变
///
/// # Safety
///
/// 调用者必须确保以下不变量：
///
/// - 若 `p` 不为 null，则 `p` 必须是先前由 `malloc()`、`calloc()`、
///   `realloc()`、`aligned_alloc()` 或 `posix_memalign()` 返回的有效指针，
///   且**尚未**被 `free()` 或 `realloc()` 释放
/// - 在调用 `free()` 或 `realloc()` 释放前，不得通过其他途径访问或修改
///   `p` 指向的内存
/// - 不得对同一指针并发调用 `realloc` 或其他释放函数
///
/// # 线程安全性
///
/// 通过内部 `malloc_impl` / `free_impl` 的锁机制保证线程安全。
/// 多线程环境下的并发 `realloc` 操作是安全的（对不同的指针）。
///
/// # 示例 (C ABI)
///
/// ```c
/// void *p = malloc(128);
/// void *new_p = realloc(p, 256);
/// if (new_p != NULL) {
///     // new_p 可能等于 p 也可能不等
/// } else {
///     // p 仍然有效，需手动释放
///     free(p);
/// }
/// ```
#[no_mangle]
pub extern "C" fn realloc(p: *mut c_void, n: usize) -> *mut c_void {
    // SAFETY: 直接委托给 mallocng 内部实现，调用者保证 p 是有效的分配指针或 NULL。
    // 参数语义 (p=NULL → 等价 malloc, n 溢出 → 返回 null+ENOMEM, 原地/移动/回退) 由 realloc_impl 处理。
    unsafe { super::mallocng::realloc::realloc_impl(p, n) }
}

/// 内部 `realloc` —— 始终使用内部分配器，不可被用户替换。
///
/// musl 其他模块（aio, dlerror 等）通过 `#define realloc __libc_realloc`
/// 使用此内部版本，确保内部分配不受用户 LD_PRELOAD 替换影响。
#[no_mangle]
pub unsafe extern "C" fn __libc_realloc(p: *mut c_void, n: usize) -> *mut c_void {
    super::mallocng::realloc::realloc_impl(p, n)
}
