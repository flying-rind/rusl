//! lcong48 — 一次性设置 48 位 LCG 的全部参数（种子、乘数、加数）。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 将 `p[0..6]` 的 7 个 u16 全部复制到 `__seed48`。

/// lcong48 — 设置全局 LCG 的全部参数为 `p` 指定的值。
///
/// `p[0..2]` = 新种子（3 个 u16，小端排列）
/// `p[3..5]` = 新乘数（3 个 u16，小端排列）
/// `p[6]`    = 新加数（1 个 u16）
///
/// # Safety
///
/// - `p` 必须指向 7 个 `u16` 的有效可读缓冲区。
/// - 修改全局可变状态（`__seed48`），非线程安全。
#[no_mangle]
pub extern "C" fn lcong48(p: *const u16) {
    // SAFETY: 调用者确保 p 指向 7 个 u16 的有效缓冲区
    // 同时修改全局可变状态 __seed48
    unsafe {
        for i in 0..7 {
            crate::__seed48::__seed48[i] = *p.add(i);
        }
    }
}
