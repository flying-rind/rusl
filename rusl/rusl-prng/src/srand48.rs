//! srand48 — 通过 32 位种子值初始化 48 位 LCG。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! musl 的 srand48 调用 seed48 设置新种子，seed48 仅修改 __seed48[0..2]
//! （种子），不修改乘数和加数。若需重置乘数加数，需额外调用 lcong48。

/// srand48 — 通过 32 位种子值初始化 48 位 LCG 的全局状态。
///
/// 设置新种子为 `{0x330E, (seedval & 0xFFFF), ((seedval >> 16) & 0xFFFF)}`。
/// 不修改乘数和加数（与 musl C 代码一致）。
///
/// # Safety
///
/// 修改全局可变状态（`__seed48`），非线程安全。
#[no_mangle]
pub unsafe extern "C" fn srand48(seedval: i64) {
    // 与 musl `seed48((unsigned short [3]){ 0x330e, seed, seed>>16 })` 一致
    crate::__seed48::__seed48[0] = 0x330e;
    crate::__seed48::__seed48[1] = seedval as u16;
    crate::__seed48::__seed48[2] = (seedval >> 16) as u16;
    // 注意：乘数 (__seed48[3..5]) 和加数 (__seed48[6]) 保持不变
    // 与 musl 代码 `seed48(...)` 行为一致（seed48 只复制 3 个 u16）
}
