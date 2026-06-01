//! drand48 / erand48 — 返回 [0.0, 1.0) 的伪随机 f64。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 使用 48 位 LCG，委托 `__rand48_step` 完成单步迭代。
//! 将 48 位结果嵌入 IEEE 754 double 的尾数（高位 52 位）得到 [1.0, 2.0)，
//! 减 1.0 映射到 [0.0, 1.0)。

/// drand48 — 使用全局 48 位 LCG 状态，返回 [0.0, 1.0) 的伪随机 f64。
///
/// 推进全局 LCG 一步，将结果除以 2^48 映射到 [0.0, 1.0) 范围。
///
/// # Safety
///
/// 读取并修改全局可变状态（`__seed48`），非线程安全。
#[no_mangle]
pub unsafe extern "C" fn drand48() -> f64 {
    erand48(&mut crate::__seed48::__seed48[0] as *mut u16)
}

/// erand48 — 使用调用者提供的种子，返回 [0.0, 1.0) 的伪随机 f64。
///
/// 不依赖全局状态，线程安全。与 `drand48()` 的算法相同但隔离。
///
/// # Safety
///
/// - `xsubi` 必须指向 3 个 `u16` 的有效可读写缓冲区（48 位种子，小端排列）。
#[no_mangle]
pub unsafe extern "C" fn erand48(xsubi: *mut u16) -> f64 {
    let step = crate::__rand48_step::__rand48_step(
        xsubi,
        &crate::__seed48::__seed48[3] as *const u16,
    );
    // 将 48 位随机值左移 4 位得到 52 位尾数，嵌入 [1.0, 2.0) 的 double
    let bits = 0x3ff0_0000_0000_0000u64 | (step << 4);
    f64::from_bits(bits) - 1.0
}
