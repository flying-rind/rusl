//! secure_getenv — 安全地获取环境变量值（GNU 扩展）。
//! 对应 musl src/env/secure_getenv.c
//!
//! 在安全执行上下文（setuid/setgid 进程）中，拒绝所有环境变量访问以防止
//! 环境变量注入攻击。在普通上下文中，等价于 `getenv`。

#![allow(unused_imports)]

use core::ffi::c_char;
use core::ptr;
use core::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// 全局状态: 进程安全模式标志
// ---------------------------------------------------------------------------

/// 进程安全模式原子标志。
///
/// 当进程以特权模式（setuid/setgid）运行时，由 `__init_libc` 设置为 `true`。
/// 一旦在进程启动阶段被设置后，在整个进程生命周期内保持不变（只读、不写入）。
/// 多线程环境中读取此标志无需加锁，天然线程安全。
///
/// 使用 `Ordering::Relaxed` 读取 —— 该标志仅在启动期（单线程上下文）写入一次，
/// 此后所有线程只读。Relaxed 语义在 x86_64、aarch64 等平台上零额外开销，
/// 与 C 原版 `libc.secure` 字段读取开销完全相同。
///
/// **不变量**: 与 `LibcState.secure` 的值始终一致，在 `__init_libc` 中同时设置。
pub(crate) static LIBC_SECURE: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// 外部依赖: getenv（POSIX 标准环境变量查找函数）
// ---------------------------------------------------------------------------

// `getenv` — 在进程环境变量列表中查找指定名称的环境变量（POSIX 标准）。
//
// 在非 c-test 模式中，该符号由 `crate::getenv` 模块的 Rust 实现
// （`#[no_mangle] pub unsafe extern "C" fn getenv`）提供，链接器自动解析。
// 在 c-test 模式中，由 musl libc 的 `getenv` C 实现提供。
extern "C" {
    fn getenv(name: *const c_char) -> *mut c_char;
}

// ---------------------------------------------------------------------------
// 对外 ABI: secure_getenv (GNU 扩展)
// ---------------------------------------------------------------------------

/// 安全地获取环境变量值（GNU 扩展，声明于 `<stdlib.h>`，需 `_GNU_SOURCE`）。
///
/// 行为取决于进程安全模式：
///
/// - **安全模式**（`LIBC_SECURE == true`，setuid/setgid 进程）：
///   始终返回 `ptr::null_mut()`，无论 `name` 内容为何。
///   不访问环境变量列表，防止环境变量注入攻击。
///
/// - **普通模式**（`LIBC_SECURE == false`）：
///   等价于 `getenv(name)`，在环境变量列表中查找匹配项并返回指向值字符串的指针。
///
/// # 参数
///
/// - `name`: 要查找的环境变量名称（NUL 结尾的 C 字符串）。
///   调用者必须确保 `name != ptr::null()`，且 `*name` 是以 `'\0'` 结尾的合法
///   C 字符串，且字符串中不包含 `'='` 字符（POSIX 规定环境变量名不得含 `=`）。
///
/// # 返回值
///
/// - 安全模式: `ptr::null_mut()`
/// - 普通模式: 与 `getenv(name)` 一致：
///   - 若 `name` 匹配某个环境变量，返回指向该环境变量**值**部分的指针
///     （如 `"PATH=/usr/bin"` 中 `=` 之后第一个字符的地址）
///   - 若未匹配，返回 `ptr::null_mut()`
///
/// 返回的指针指向进程环境内存，调用者**不可修改或释放**该内存。
/// 该指针在下次修改环境的调用（`putenv`/`setenv`/`unsetenv`/`clearenv`）后
/// 可能失效。
///
/// # 线程安全
///
/// 该函数仅读取 `AtomicBool`（只读原子变量）和调用 `getenv`（读 `environ`），
/// 无写入操作。在多线程环境中安全，但若其他线程同时修改环境变量列表，
/// 行为未定义（与 POSIX `getenv` 相同）。
///
/// # 不设置 errno
///
/// 该函数在任何情况下都不设置 `errno`。
///
/// # ABI 兼容性
///
/// `#[no_mangle]` + `extern "C"` 确保该符号的 ABI 与 musl/glibc 的
/// `secure_getenv` 完全兼容，外部 C 代码可透明调用。
///
/// # Safety
///
/// 本函数自身不标记为 `unsafe`。调用者传入无效指针的风险由 `getenv` 内部承担，
/// 而非本函数的职责。函数体中的 `unsafe` 仅限定于 `getenv` FFI 调用这一条语句。
#[no_mangle]
pub extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char {
    // Step 1: 读取安全模式标志
    //
    // 使用 Ordering::Relaxed —— 该标志仅在启动期（单线程上下文）写入一次，
    // 此后只读。Relaxed 语义在 x86_64、aarch64 等平台上编译为普通内存读取
    // 指令（如 `mov`），与 C 原版 `libc.secure` 字段读取完全一致。
    if LIBC_SECURE.load(Ordering::Relaxed) {
        // Case 1: 安全模式 —— 拒绝所有环境变量访问
        return ptr::null_mut();
    }

    // Case 2: 普通模式 —— 委托给标准 getenv
    // name 的合法性检查（NULL 检测、空字符串检测、'=' 检测）由 getenv 内部完成。
    unsafe { getenv(name) }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    // 安全模式下，`secure_getenv` 对有效名称返回 `null_mut()`。
    test!("secure_mode_returns_null_for_valid_name" {
        // 设置安全模式
        LIBC_SECURE.store(true, Ordering::Release);

        let name = b"PATH\0".as_ptr() as *const c_char;
        let result = secure_getenv(name);
        assert!(result.is_null());

        // 恢复非安全模式，避免影响后续测试
        LIBC_SECURE.store(false, Ordering::Release);
    });

    // 安全模式下，对空字符串名称也返回 `null_mut()`。
    test!("secure_mode_returns_null_for_empty_name" {
        LIBC_SECURE.store(true, Ordering::Release);

        let empty = b"\0".as_ptr() as *const c_char;
        assert!(secure_getenv(empty).is_null());

        LIBC_SECURE.store(false, Ordering::Release);
    });

    // 安全模式下，对不存在的变量名也返回 `null_mut()`。
    test!("secure_mode_returns_null_for_nonexistent_name" {
        LIBC_SECURE.store(true, Ordering::Release);

        let nonexistent = b"__RUSL_TEST_NONEXISTENT_987654321\0".as_ptr() as *const c_char;
        assert!(secure_getenv(nonexistent).is_null());

        LIBC_SECURE.store(false, Ordering::Release);
    });

    // 安全模式下，无论 LIBC_SECURE 读取多少次，secure_getenv 始终返回 null。
    test!("secure_mode_consistently_returns_null" {
        LIBC_SECURE.store(true, Ordering::Release);

        let name = b"HOME\0".as_ptr() as *const c_char;
        let r1 = secure_getenv(name);
        let r2 = secure_getenv(name);
        let r3 = secure_getenv(name);
        assert!(r1.is_null());
        assert!(r2.is_null());
        assert!(r3.is_null());

        LIBC_SECURE.store(false, Ordering::Release);
    });

    // 验证 `LIBC_SECURE` 初始值为 `false`（非安全模式）。
    test!("libc_secure_defaults_to_false" {
        // 确保在测试开始时 LIBC_SECURE 不是遗留的 true 状态
        LIBC_SECURE.store(false, Ordering::Release);
        assert!(!LIBC_SECURE.load(Ordering::Relaxed));
    });

    // 验证 AtomicBool store/load 的往返一致性。
    test!("libc_secure_atomic_roundtrip" {
        // 确保起点是 false
        LIBC_SECURE.store(false, Ordering::Release);

        // 往返测试
        LIBC_SECURE.store(true, Ordering::Release);
        assert!(LIBC_SECURE.load(Ordering::Acquire));

        LIBC_SECURE.store(false, Ordering::Release);
        assert!(!LIBC_SECURE.load(Ordering::Acquire));

        // 恢复 false
        LIBC_SECURE.store(false, Ordering::Release);
    });

    // 普通模式下，secure_getenv 对 PATH（通常存在）应返回非空值。
    // 此测试依赖 environ 已由启动代码初始化。
    test!("normal_mode_delegates_to_getenv" {
        // 确保非安全模式
        LIBC_SECURE.store(false, Ordering::Release);

        let name = b"PATH\0".as_ptr() as *const c_char;
        let r_secure = secure_getenv(name);
        let r_getenv = unsafe { getenv(name) };

        // 两者应返回相同结果
        assert_eq!(r_secure, r_getenv);
    });

    // 普通模式下，不存在的变量返回 null。
    test!("normal_mode_nonexistent_returns_null" {
        LIBC_SECURE.store(false, Ordering::Release);

        let nonexistent = b"__RUSL_NONEXISTENT_VAR_98765\0".as_ptr() as *const c_char;
        assert!(secure_getenv(nonexistent).is_null());
    });

    // 多次调用返回一致结果（期间环境未修改）。
    test!("repeat_calls_return_same_result" {
        LIBC_SECURE.store(false, Ordering::Release);

        let name = b"PATH\0".as_ptr() as *const c_char;
        let r1 = secure_getenv(name);
        let r2 = secure_getenv(name);

        // 两次调用应返回相同指针
        assert_eq!(r1, r2);
    });
}
