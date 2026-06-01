// errno implementation.
//
// Corresponds to musl's src/errno/__errno_location.c
//
// In musl, errno is stored in the pthread struct: &__pthread_self()->errno_val.
// Since Stage 0 has no pthread yet, we use a simple static.
// TODO(Stage 5): Replace with TLS via thread pointer register when pthread is ready.
//
// We provide both __errno_location (C11) and ___errno_location (GNU weak alias).

use core::ffi::c_int;

// Stage 0: simple global errno (NOT thread-safe).
// Stage 5 will replace this with per-thread storage.
static mut ERRNO: c_int = 0;

// ---------------------------------------------------------------------------
// errno 常量 — POSIX errno 值
// ---------------------------------------------------------------------------

/// EINVAL — 参数无效 (Invalid argument)。
///
/// POSIX.1-2001 定义。Linux x86_64 / aarch64 上 errno 值 = 22。
/// 当函数参数不满足约束条件时（如环境变量名为空或包含 '='），
/// 通过 [`set_errno`] 设置此值。
pub const EINVAL: c_int = 22;

// ---------------------------------------------------------------------------
// set_errno — 辅助函数
// ---------------------------------------------------------------------------

/// 设置当前线程的 errno 值。
///
/// # Safety
///
/// - 调用者必须确保在单线程环境下调用，或在多线程环境下正确同步。
/// - Stage 0 使用全局静态 errno（非线程安全），与 POSIX 关于环境变量
///   操作的线程安全约束一致。
///
/// # 实现
///
/// 内部调用 [`__errno_location`] 获取 errno 指针后直接写入。
pub unsafe fn set_errno(val: c_int) {
    *__errno_location() = val;
}

/// Return a pointer to the calling thread's errno variable.
///
/// C signature: int *__errno_location(void);
#[no_mangle]
pub unsafe extern "C" fn __errno_location() -> *mut c_int {
    core::ptr::addr_of_mut!(ERRNO)
}

/// Weak alias for GNU compatibility.
/// In C: weak_alias(__errno_location, ___errno_location);
#[no_mangle]
pub unsafe extern "C" fn ___errno_location() -> *mut c_int {
    core::ptr::addr_of_mut!(ERRNO)
}