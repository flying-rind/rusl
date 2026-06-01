//! mrand48 / jrand48 — 返回 [-2^31, 2^31) 的有符号伪随机 i64。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 委托 `__rand48_step` 完成 48 位 LCG 单步迭代，然后取高 32 位
//! 并以有符号值解释。

/// mrand48 — 使用全局 48 位 LCG 状态，返回 [-2^31, 2^31) 的有符号伪随机 i64。
///
/// 推进全局 LCG 一步，取结果高 32 位作为有符号值。
///
/// # Safety
///
/// 读取并修改全局可变状态（`__seed48`），非线程安全。
#[no_mangle]
pub unsafe extern "C" fn mrand48() -> i64 {
    jrand48(&mut crate::__seed48::__seed48[0] as *mut u16)
}

/// jrand48 — 使用调用者提供的种子，返回 [-2^31, 2^31) 的有符号伪随机 i64。
///
/// 不依赖全局状态，线程安全。
///
/// # Safety
///
/// - `xsubi` 必须指向 3 个 `u16` 的有效可读写缓冲区（48 位种子，小端排列）。
#[no_mangle]
pub unsafe extern "C" fn jrand48(xsubi: *mut u16) -> i64 {
    let step = crate::__rand48_step::__rand48_step(
        xsubi,
        &crate::__seed48::__seed48[3] as *const u16,
    );
    // 取高 32 位，转为 i32（有符号），再符号扩展为 i64
    (step >> 16) as i32 as i64
}
