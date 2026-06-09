//! \_\_errno\_location — Rust 实现 musl libc errno 线程局部存储访问器。
//!
//! Stage 0 使用全局 `static mut ERRNO` (非线程安全)。
//! Stage 5 将迁移至 pthread 结构体内部的 per-thread 存储。
//!
//! 导出的符号:
//! - `__errno_location`  — C11 标准 errno 位置访问器
//! - `___errno_location` — GNU 弱别名, 行为与 `__errno_location` 完全一致
//! - `set_errno`         — 设置 errno 的辅助函数
//! - `EINVAL`            — POSIX errno 常量

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// Stage 0: 全局 errno 存储 (非线程安全)
// ---------------------------------------------------------------------------

/// 全局 errno 变量。
///
/// # Safety
///
/// 在单线程环境中读写是安全的。多线程环境中需要同步或迁移至 Stage 5 的
/// per-thread 存储。
static mut ERRNO: c_int = 0;

// ---------------------------------------------------------------------------
// errno 常量 — POSIX errno 值
// ---------------------------------------------------------------------------

/// EINVAL — 参数无效 (Invalid argument)。
///
/// POSIX.1-2001 定义。Linux x86_64 / aarch64 上 errno 值 = 22。
pub const EINVAL: c_int = 22;

// ---------------------------------------------------------------------------
// set_errno — 辅助函数
// ---------------------------------------------------------------------------

/// 设置当前线程的 errno 值。
///
/// # Safety
///
/// - 调用者必须确保在单线程环境下调用，或在多线程环境下正确同步。
/// - Stage 0 使用全局静态 errno（非线程安全）。
pub unsafe fn set_errno(val: c_int) {
    *__errno_location() = val;
}

// ---------------------------------------------------------------------------
// __errno_location — 返回当前线程 errno 变量的地址
// ---------------------------------------------------------------------------

/// 返回指向当前"线程" errno 变量的指针。
///
/// # 返回值
///
/// 始终返回有效的非空 `*mut c_int` 指针:
/// - Stage 0: 所有调用返回同一个全局静态变量 [`ERRNO`] 的地址。
/// - Stage 5: 不同线程返回不同的 per-thread 地址。
pub extern "C" fn __errno_location() -> *mut c_int {
    core::ptr::addr_of_mut!(ERRNO)
}

// ---------------------------------------------------------------------------
// ___errno_location — GNU 弱别名
// ---------------------------------------------------------------------------

/// GNU 兼容的 errno 位置访问器弱别名。
///
/// 与 [`__errno_location`] 返回完全相同的地址，行为完全一致。
/// 在 musl 中通过 `weak_alias(__errno_location, ___errno_location)` 实现。
pub extern "C" fn ___errno_location() -> *mut c_int {
    core::ptr::addr_of_mut!(ERRNO)
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::test;
    use crate::errno::__errno_location;
    use crate::errno::___errno_location;
    use crate::errno::set_errno;
    use crate::errno::EINVAL;

    test!("test_errno_location_non_null" {
        let p = __errno_location();
        assert!(!p.is_null(), "__errno_location returned null pointer");
    });

    test!("test_both_aliases_same_address" {
        let p1 = __errno_location();
        let p2 = ___errno_location();
        assert_eq!(p1, p2, "aliases must return the same address");
    });

    test!("test_errno_read_default_zero" {
        let p = __errno_location();
        let val = unsafe { core::ptr::read(p) };
        assert_eq!(val, 0, "initial errno should be 0");
    });

    test!("test_errno_write_read" {
        let p = __errno_location();
        unsafe { core::ptr::write(p, 42) };
        let val = unsafe { core::ptr::read(p) };
        assert_eq!(val, 42, "errno write/read mismatch");
    });

    test!("test_errno_aliases_write" {
        let p1 = __errno_location();
        let p2 = ___errno_location();
        unsafe { core::ptr::write(p2, 99) };
        let val = unsafe { core::ptr::read(p1) };
        assert_eq!(val, 99, "aliases should share the same storage");
    });

    test!("test_errno_reset" {
        let p = __errno_location();
        unsafe { core::ptr::write(p, 0) };
        let val = unsafe { core::ptr::read(p) };
        assert_eq!(val, 0, "errno reset failed");
    });

    test!("test_set_errno" {
        unsafe { set_errno(42) };
        let val = unsafe { core::ptr::read(__errno_location()) };
        assert_eq!(val, 42, "set_errno failed");
    });

    test!("test_einval_constant" {
        assert_eq!(EINVAL, 22, "EINVAL should be 22");
    });
}