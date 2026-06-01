//! rand / srand — 标准 C 库的简单伪随机数生成器。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 使用 64 位 LCG: `seed = 6364136223846793005 * seed + 1`，返回高 31 位。

// RAND_MAX 常量，与 C 头文件中 #define RAND_MAX (0x7fffffff) 一致。
#[allow(dead_code)]
pub const RAND_MAX: i32 = 0x7fffffff;

/// 全局种子（对应于 musl 中 `static uint64_t seed`）。
static mut SEED: u64 = 0;

/// rand — 返回 [0, RAND_MAX] 的伪随机整数。
///
/// 使用 64 位 LCG: `seed = 6364136223846793005 * seed + 1`，返回高 31 位。
///
/// # Safety
///
/// 读取并修改全局可变种子 (`static mut seed: u64`)，非线程安全。
#[no_mangle]
pub unsafe extern "C" fn rand() -> i32 {
    SEED = 6364136223846793005u64.wrapping_mul(SEED).wrapping_add(1);
    (SEED >> 33) as i32
}

/// srand — 设置全局种子为 `(seed - 1) as u64`。
///
/// # Safety
///
/// 修改全局可变种子，非线程安全。
#[no_mangle]
pub unsafe extern "C" fn srand(seed: u32) {
    SEED = (seed as u64).wrapping_sub(1);
}
