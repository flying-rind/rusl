//! toascii — 将字符强制转换为 7 位 ASCII。
//! 对应 musl src/ctype/toascii.c
//!
//! **此函数已过时，不应在新代码中使用。** 保留仅为 BSD/POSIX 兼容性。

use core::ffi::c_int;

/// 将字符 c 强制转换为 7 位 ASCII（清除第 7 位及以上所有位）。
///
/// 等价于 `c & 0x7f`，将值映射到 0-127 的 ASCII 范围。
///
/// # 参数
///
/// * `c` - 任意整数值（类型为 `c_int`）
///
/// # 返回
///
/// 返回 `c & 0x7f`，即仅保留低 7 位。
///
/// # 过时说明
///
/// 此函数已过时（POSIX 标记为 LEGACY）。新代码应直接使用按位与操作。
#[no_mangle]
pub extern "C" fn toascii(c: c_int) -> c_int {
    c & 0x7f
}
