//! random / srandom / initstate / setstate — BSD random() 族。
//! 使用滞后斐波那契生成器（LFSR），周期远大于 rand()。
//! [Visibility]: Public — 对外导出 (extern "C" ABI)
//!
//! 内部使用 AtomicBool 自旋锁保护全局状态，与 C 中 `LOCK(lock)` 对应。
//! 状态表包含一个元数据字（存储 n/i/j）和 n 个状态字。

use core::hint;
use core::sync::atomic::{AtomicBool, Ordering};

// ─── 自旋锁 ──────────────────────────────────────────────────────

static LOCK: AtomicBool = AtomicBool::new(false);

fn lock() {
    while LOCK.swap(true, Ordering::Acquire) {
        while LOCK.load(Ordering::Relaxed) {
            hint::spin_loop();
        }
    }
}

fn unlock() {
    LOCK.store(false, Ordering::Release);
}

// ─── 默认状态表 ──────────────────────────────────────────────────
// 与 musl `static uint32_t init[] = {...}` 完全一致。

static mut STATE_BUF: [u32; 32] = [
    0x00000000, 0x5851f42d, 0xc0b18ccf, 0xcbb5f646,
    0xc7033129, 0x30705b04, 0x20fd5db4, 0x9a8b7f78,
    0x502959d8, 0xab894868, 0x6c0356a7, 0x88cdb7ff,
    0xb477d43f, 0x70a3a52b, 0xa8e4baf1, 0xfd8341fc,
    0x8ae16fd9, 0x742d2f7a, 0x0d1f0796, 0x76035e09,
    0x40f7702c, 0x6fa72ca5, 0xaaa84157, 0x58a0df74,
    0xc74a0364, 0xae533cc4, 0x04185faf, 0x6de3b115,
    0x0cab8628, 0xf043bfa4, 0x398150e9, 0x37521657,
];

// ─── 全局状态变量 ────────────────────────────────────────────────
// 对应 C: `static int n = 31, i = 3, j = 0; static uint32_t *x = init+1;`

static mut STATE_N: i32 = 31; // 度（状态字个数）
static mut STATE_I: i32 = 3;  // 滞后 i
static mut STATE_J: i32 = 0;  // 滞后 j
/// 状态指针：指向当前状态表的第 1 个元素（跳过元数据字）。
/// 即 C 中的 `static uint32_t *x = init+1`。
static mut STATE_X: *mut u32 = core::ptr::null_mut();

// ─── 内部工具函数 ────────────────────────────────────────────────

/// 31 位 LCG：`(1103515245 * x + 12345) & 0x7fffffff`
fn lcg31(x: u32) -> u32 {
    (1103515245u32.wrapping_mul(x).wrapping_add(12345)) & 0x7fffffff
}

/// 64 位 LCG：`(6364136223846793005 * x + 1)`
fn lcg64(x: u64) -> u64 {
    6364136223846793005u64.wrapping_mul(x).wrapping_add(1)
}

/// 确保 STATE_X 已初始化（指向默认状态表）。
///
/// 对应 C 中 `x = init+1` 的惰性初始化。
unsafe fn ensure_init() {
    if STATE_X.is_null() {
        STATE_X = &mut STATE_BUF[0] as *mut u32;
        STATE_X = STATE_X.add(1);
    }
}

/// 保存当前状态：将 n/i/j 打包写入元数据字（x[-1]），
/// 返回指向元数据字的指针。对应 C 的 `savestate()`。
unsafe fn savestate() -> *mut u32 {
    let meta = ((STATE_N as u32) << 16) | ((STATE_I as u32) << 8) | (STATE_J as u32);
    STATE_X.sub(1).write_unaligned(meta);
    STATE_X.sub(1)
}

/// 从元数据字恢复 n/i/j，设置 STATE_X。
/// `state` 指向元数据字（即状态表的起始位置）。
/// 对应 C 的 `loadstate(uint32_t *state)`。
unsafe fn loadstate(state: *mut u32) {
    STATE_X = state.add(1);
    let meta = state.read_unaligned();
    STATE_N = (meta >> 16) as i32;
    STATE_I = ((meta >> 8) & 0xff) as i32;
    STATE_J = (meta & 0xff) as i32;
}

/// 根据种子重新填充整个状态表。
/// 对应 C 的 `static void __srandom(unsigned seed)`。
///
/// - 若 n == 0（退化状态），仅设 x[0] = seed。
/// - 否则用 lcg64 填充 x[0..n-1]，并确保至少一个奇数。
unsafe fn __srandom(seed: u32) {
    if STATE_N == 0 {
        STATE_X.write_unaligned(seed as u64 as u32);
        return;
    }
    // 设置滞后参数
    STATE_I = if STATE_N == 31 || STATE_N == 7 { 3 } else { 1 };
    STATE_J = 0;
    // 用 lcg64 填充
    let mut s: u64 = seed as u64;
    for k in 0..STATE_N as usize {
        s = lcg64(s);
        STATE_X.add(k).write_unaligned((s >> 32) as u32);
    }
    // 确保至少一个奇数
    STATE_X.write_unaligned(STATE_X.read_unaligned() | 1);
}

// ─── 公有 API ────────────────────────────────────────────────────

/// random — 返回 [0, 2^31-1] 的非负伪随机 i64。
///
/// 使用 LFSR 混洗: `state[i] += state[j]; return state[i] >> 1`。
/// 内部使用自旋锁保护全局状态。
///
/// # Safety
///
/// 读取并修改全局可变状态，受自旋锁保护但调用者仍需确保无数据竞争。
#[no_mangle]
pub extern "C" fn random() -> i64 {
    // SAFETY: 访问 static mut 全局状态，受自旋锁保护
    unsafe {
        lock();
        ensure_init();

        let k: u32;
        if STATE_N == 0 {
            // 退化模式：直接使用 31 位 LCG
            let val = lcg31(STATE_X.read_unaligned());
            STATE_X.write_unaligned(val);
            k = val;
        } else {
            // 正常模式：滞后斐波那契
            let i = STATE_I as usize;
            let j = STATE_J as usize;
            let new_val = (STATE_X.add(i).read_unaligned()).wrapping_add(STATE_X.add(j).read_unaligned());
            STATE_X.add(i).write_unaligned(new_val);
            k = new_val >> 1;

            // 推进索引
            STATE_I += 1;
            if STATE_I == STATE_N {
                STATE_I = 0;
            }
            STATE_J += 1;
            if STATE_J == STATE_N {
                STATE_J = 0;
            }
        }

        unlock();
        k as i64
    }
}

/// srandom — 为 random() 设置种子。
///
/// 使用 64 位 LCG 展开种子填充状态表并预混洗。
///
/// # Safety
///
/// 修改全局可变状态，受自旋锁保护。
#[no_mangle]
pub extern "C" fn srandom(seed: u32) {
    // SAFETY: 访问 static mut 全局状态，受自旋锁保护
    unsafe {
        lock();
        ensure_init();
        __srandom(seed);
        unlock();
    }
}

/// initstate — 初始化 random() 的状态表并切换至此状态。
///
/// 状态表大小由 `n` 决定：
/// - `n >= 256`: deg = 63
/// - `n >= 128`: deg = 31
/// - `n >= 64` : deg = 15
/// - `n >= 32` : deg = 7
/// - `n >= 8`  : deg = 0（退化模式）
/// - `n < 8`   : 返回 null（出错）
///
/// # Safety
///
/// - `state` 必须指向至少 `n` 字节的有效可写缓冲区。
/// - 修改并切换全局可变状态。
///
/// # Returns
///
/// - 成功时返回指向旧状态缓冲区的指针。
/// - `n < 8` 时返回 null 指针。
#[no_mangle]
pub extern "C" fn initstate(seed: u32, state: *mut u8, n: usize) -> *mut u8 {
    if n < 8 {
        return core::ptr::null_mut();
    }
    // SAFETY: 访问 static mut 全局状态，受自旋锁保护；调用者确保 state 有效
    unsafe {
        lock();
        ensure_init();

        // 保存旧状态元数据并获取旧状态指针
        let old = savestate();

        // 根据 buffer 大小选择度
        STATE_N = if n < 32 {
            0
        } else if n < 64 {
            7
        } else if n < 128 {
            15
        } else if n < 256 {
            31
        } else {
            63
        };

        // 设置新状态表指针（跳过一个 u32 作为元数据字）
        STATE_X = (state as *mut u32).add(1);

        // 用种子填充新状态表
        __srandom(seed);

        // 将新状态的元数据写入新状态表
        savestate();

        unlock();
        old as *mut u8
    }
}

/// setstate — 切换 random() 的状态表至先前保存的状态。
///
/// # Safety
///
/// - `state` 必须由先前的 `initstate` 初始化，指向有效的状态表。
/// - 修改全局可变状态。
///
/// # Returns
///
/// 返回指向旧状态缓冲区的指针。
#[no_mangle]
pub extern "C" fn setstate(state: *mut u8) -> *mut u8 {
    // SAFETY: 访问 static mut 全局状态，受自旋锁保护；调用者确保 state 有效
    unsafe {
        lock();
        ensure_init();

        // 保存当前状态并获取旧状态指针
        let old = savestate() as *mut u8;

        // 加载新状态
        loadstate(state as *mut u32);

        unlock();
        old
    }
}
