//! reallocarray — 带整数溢出检查的安全数组内存重分配（OpenBSD BSD 扩展）
//!
//! 该函数是 `realloc(ptr, m * n)` 的安全替代品。在 `m * n` 乘法溢出 `usize` 时，
//! 返回 `null` 并设置 `errno = ENOMEM`，而不是返回一个错误且可能极小的分配结果。
//!
//! 原始 C 签名:
//! ```c
//! void *reallocarray(void *ptr, size_t m, size_t n);
//! ```
//!
//! 对应 musl 源码: `src/malloc/reallocarray.c`

use core::ffi::c_void;
use rusl_errno::__errno_location;

/// 带整数溢出检查的安全数组内存重分配。
///
/// 分配 `m * n` 个字节的内存（即 `m` 个元素，每个元素 `n` 字节），
/// 并在乘法溢出 `usize` 时返回 `null` 并设置 `errno = ENOMEM`。
///
/// # 行为
///
/// - **溢出情况**: 若 `n != 0` 且 `m > usize::MAX / n`，则 `errno = ENOMEM`，
///   返回 `core::ptr::null_mut()`，`ptr` 指向的原始内存块保持未修改。
/// - **无溢出**: 委托给 `realloc(ptr, m * n)`，透传其返回值和 errno。
/// - **`ptr == null`**: 等价于 `malloc(m * n)`。
/// - **`m * n == 0`**: 等价于 `realloc(ptr, 0)`，行为由底层 `realloc` 实现决定。
///
/// # Safety
///
/// 调用者必须确保：
/// - 若 `ptr` 非 null，则它必须是先前由 `malloc`、`calloc`、`realloc`
///   或 `reallocarray` 返回的有效指针，且尚未被 `free` 或 `realloc` 释放。
///
/// # 返回值
///
/// - 成功: 指向新分配内存块（至少 `m * n` 字节）的指针
/// - 失败: `core::ptr::null_mut()`，且 `errno` 被设置为 `ENOMEM`
///
/// # 示例
///
/// ```rust,no_run,ignore
/// # use rusl::malloc::reallocarray;
/// use core::ffi::c_void;
///
/// unsafe {
///     // 分配 10 个元素，每个 8 字节
///     let ptr = reallocarray(core::ptr::null_mut(), 10, 8);
///     assert!(!ptr.is_null());
///     // ... 使用 ptr ...
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn reallocarray(
    ptr: *mut c_void,
    m: usize,
    n: usize,
) -> *mut c_void {
    // 溢出检测: 若 n != 0 且 m * n 会溢出 usize，返回 null 并设置 errno = ENOMEM。
    if n != 0 && m > usize::MAX / n {
        unsafe {
            *__errno_location() = super::ENOMEM;
        }
        return core::ptr::null_mut();
    }

    // 安全乘法: 已确保不溢出，但使用 wrapping_mul 避免 debug 构建中潜在的溢出 panic。
    let size = m.wrapping_mul(n);
    super::realloc::realloc(ptr, size)
}
