//! abs —— 计算 int 的绝对值。对外导出 C ABI 兼容的 `abs` 符号。

#![allow(unused_imports, unused_variables)]

/// 计算 `a` 的绝对值。
///
/// # Safety
///
/// - `a` 可以为任意 i32 值，但当 `a == i32::MIN` 时行为未定义（无法用 i32 表示）。
///
/// # 返回值
///
/// - `a >= 0` 时返回 `a`。
/// - `a < 0` 时返回 `-a`。
#[no_mangle]
pub unsafe extern "C" fn abs(a: i32) -> i32 {
    if a > 0 { a } else { a.wrapping_neg() }
}
