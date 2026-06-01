//! rand_r — 可重入版本的 `rand()`, 含 MT 调和函数 (temper)。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 注意: musl 的 rand_r 并非裸 LCG，而是额外经过 4 步 Mersenne Twister 风格
//! 的调和 (temper) 变换，再将结果右移 1 位。

// RAND_MAX 常量，与 C 头文件中 #define RAND_MAX (0x7fffffff) 一致。
// 仅在测试中使用，故 allow(dead_code)。
#[allow(dead_code)]
const RAND_MAX: i32 = 0x7fffffff;

/// Mersenne Twister 风格的调和变换。
///
/// 与 musl `src/prng/rand_r.c` 中的 `static unsigned temper(unsigned x)`
/// 完全一致。
fn temper(x: u32) -> u32 {
    let mut t = x;
    t ^= t >> 11;
    t ^= (t << 7) & 0x9D2C5680;
    t ^= (t << 15) & 0xEFC60000;
    t ^= t >> 18;
    t
}

/// rand_r — 可重入伪随机数生成器。
///
/// 先执行 32 位 LCG: `*seed = (*seed).wrapping_mul(1103515245).wrapping_add(12345);`
/// 再对结果施以 temper 调和，最后右移 1 位返回。
///
/// 与 `rand()` 不共享状态；所有状态由调用者通过 `seed` 管理。
///
/// # Safety
///
/// - `seed` 必须指向调用者维护的有效可读写 `u32` 变量，不能为空指针。
#[no_mangle]
pub unsafe extern "C" fn rand_r(seed: *mut u32) -> i32 {
    let new_seed = (*seed).wrapping_mul(1103515245).wrapping_add(12345);
    *seed = new_seed;
    (temper(new_seed) >> 1) as i32
}
