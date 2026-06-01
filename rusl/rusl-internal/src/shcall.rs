//! shcall 模块 — SuperH 架构 PLT 隔离转发包装器。
//!
//! 本模块定义 `__shcall` 函数，是 musl 在 SuperH (sh) 架构上的
//! 函数指针调用转发层。在 rusl（Rust 实现）中，由于 Rust 编译器
//! 不产生 PLT 相关的重入问题，此函数作为透明转发器存在。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。若 rusl 不需要支持
//! SuperH 架构，此模块可被条件编译排除。

use core::ffi::{c_int, c_void};

/// SuperH 架构 PLT 解耦的转发包装器。
///
/// 将函数指针调用从共享库 PLT 延迟绑定路径中解耦。
/// 在 Rust 中此函数为透明转发：`__shcall(arg, func)` 等价于 `func(arg)`。
///
/// # Safety
///
/// * `func` 必须为非空的函数指针
/// * `arg` 的类型必须与 `func` 期望的参数类型兼容
///
/// # 参数
///
/// * `arg` - 传递给 `func` 的参数
/// * `func` - 被调用的函数指针
///
/// # 返回值
///
/// `func(arg)` 的返回值
#[inline(always)]
pub unsafe fn __shcall(
    arg: *mut c_void,
    func: Option<unsafe extern "C" fn(*mut c_void) -> c_int>,
) -> c_int {
    // 透明转发：在 Rust 中 PLT 解耦无实际意义，直接调用即可。
    match func {
        Some(f) => f(arg),
        None => 0, // SIG_DFL 行为：返回 0 表示默认处理
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use core::ffi::{c_int, c_void};
    use super::__shcall;

    /// 测试辅助函数：将参数指针解释为 i32 并返回其值
    unsafe extern "C" fn identity(arg: *mut c_void) -> c_int {
        arg as c_int
    }

    /// 测试辅助函数：将参数指针解释为 i32 并返回其双倍值
    unsafe extern "C" fn double(arg: *mut c_void) -> c_int {
        (arg as c_int) * 2
    }

    test!("shcall_transparent_forward" {
        unsafe {
            let result = __shcall(42 as *mut c_void, Some(identity));
            assert_eq!(result, 42);
        }
    });

    test!("shcall_double_forward" {
        unsafe {
            let result = __shcall(21 as *mut c_void, Some(double));
            assert_eq!(result, 42);
        }
    });

    test!("shcall_zero_arg" {
        unsafe {
            let result = __shcall(0 as *mut c_void, Some(identity));
            assert_eq!(result, 0);
        }
    });

    test!("shcall_none_func_returns_zero" {
        unsafe {
            let result = __shcall(42 as *mut c_void, None);
            assert_eq!(result, 0);
        }
    });
}