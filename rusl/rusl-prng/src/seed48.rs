//! seed48 — 设置 48 位 LCG 种子并返回旧种子。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 保存旧种子至内部静态缓冲区，返回该缓冲区的可变指针。
//! 后续调用 `seed48` 会覆盖此缓冲区。
//!
//! 注意：musl 的 seed48 **只修改种子**（前 3 个 u16），
//! 不重置乘数和加数。若要同时重置参数，请调用 `srand48`。

/// 旧种子保存缓冲区（对应 C 中 `static unsigned short p[3]`）。
static mut OLD_SEED: [u16; 3] = [0; 3];

/// seed48 — 设置 48 位 LCG 的新种子，不修改乘数和加数。
///
/// 保存旧种子至内部静态缓冲区，返回该缓冲区的可变指针。
/// 后续调用会覆盖此缓冲区。
///
/// # Safety
///
/// - `seed16v` 必须指向 3 个 `u16` 的有效可读缓冲区（新种子值）。
/// - 修改全局可变状态（`__seed48`），非线程安全。
/// - 返回的指针指向内部静态缓冲区，后续调用或并发访问会导致数据竞争。
#[no_mangle]
pub unsafe extern "C" fn seed48(seed16v: *const u16) -> *mut u16 {
    // 保存旧种子
    OLD_SEED[0] = crate::__seed48::__seed48[0];
    OLD_SEED[1] = crate::__seed48::__seed48[1];
    OLD_SEED[2] = crate::__seed48::__seed48[2];

    // 设置新种子（仅修改前 3 个 u16，遵循 musl 行为）
    crate::__seed48::__seed48[0] = *seed16v;
    crate::__seed48::__seed48[1] = *seed16v.add(1);
    crate::__seed48::__seed48[2] = *seed16v.add(2);

    // 注意：乘数 (__seed48[3..5]) 和加数 (__seed48[6]) 保持不变
    // musl 的 seed48 不重置参数，与 glibc 不同

    core::ptr::addr_of_mut!(OLD_SEED) as *mut u16
}
