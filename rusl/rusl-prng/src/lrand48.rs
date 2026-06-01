//! lrand48 / nrand48 — 返回 [0, 2^31) 的非负伪随机 i64。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 委托 `__rand48_step` 完成 48 位 LCG 单步迭代，然后取高 31 位。

/// lrand48 — 使用全局 48 位 LCG 状态，返回 [0, 2^31) 的非负伪随机 i64。
///
/// 推进全局 LCG 一步，取结果高 31 位（移位 17 位后与 0x7FFFFFFF 求与）。
///
/// # Safety
///
/// 读取并修改全局可变状态（`__seed48`），非线程安全。
#[no_mangle]
pub unsafe extern "C" fn lrand48() -> i64 {
    nrand48(&mut crate::__seed48::__seed48[0] as *mut u16)
}

/// nrand48 — 使用调用者提供的种子，返回 [0, 2^31) 的非负伪随机 i64。
///
/// 不依赖全局状态，线程安全。
///
/// # Safety
///
/// - `xsubi` 必须指向 3 个 `u16` 的有效可读写缓冲区（48 位种子，小端排列）。
#[no_mangle]
pub unsafe extern "C" fn nrand48(xsubi: *mut u16) -> i64 {
    (crate::__rand48_step::__rand48_step(
        xsubi,
        &crate::__seed48::__seed48[3] as *const u16,
    ) >> 17) as i64
}
