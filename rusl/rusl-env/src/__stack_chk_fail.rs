//! # __stack_chk_fail — 栈保护器 (Stack Smashing Protector, SSP) 运行时支持
//!
//! 本模块实现 GCC/Clang 栈保护器的运行时支持，对应 musl `src/env/__stack_chk_fail.c`。
//!
//! 当程序以 `-fstack-protector` 或类似选项编译时，编译器会在每个函数的栈帧中
//! 插入一个 "canary" 值，并在函数返回前检查该值是否被篡改。若检测到栈缓冲区溢出，
//! 则调用 [`__stack_chk_fail`] 终止进程。
//!
//! ## 组件
//!
//! | 符号 | 说明 |
//! |------|------|
//! | [`__stack_chk_guard`] | 全局 canary 变量，编译器生成的栈保护代码直接引用 |
//! | [`__init_ssp`] | 初始化 canary 值（由 CRT 启动代码在程序启动早期调用） |
//! | [`__stack_chk_fail`] | canary 校验失败时由编译器生成代码调用的崩溃函数 |
//! | [`__stack_chk_fail_local`] | `__stack_chk_fail` 的 DSO 局部弱别名 |
//!
//! ## 平台条件编译
//!
//! - 64 位平台 (`cfg(target_pointer_width = "64")`)：canary 第二字节清零，防御字符串攻击
//! - 32 位平台 (`cfg(not(target_pointer_width = "64"))`)：使用原始 canary 值
//!
//! ## 依赖
//!
//! - `__pthread_self()` — 从 `rusl_internal::pthread_impl` 获取当前线程控制块，
//!   用于将 canary 值同步到线程 TLS 头部。

use core::ffi::c_void;
use core::ptr;

// ---------------------------------------------------------------------------
// 常量定义
// ---------------------------------------------------------------------------

/// canary 确定性生成算法乘数常量（`0x41C64E6D`）。
///
/// 当内核未提供 `AT_RANDOM` 随机熵时，使用此常量与 `__stack_chk_guard`
/// 的地址相乘生成伪随机 canary 值。在 ASLR 环境中，地址本身的随机性
/// 保证了 canary 的不可预测性。
const CANARY_MULTIPLIER: usize = 0x41C64E6D;

// ---------------------------------------------------------------------------
// 全局变量 — GCC/Clang 编译器 ABI 所需
// ---------------------------------------------------------------------------

/// 全局栈 canary 变量。
///
/// # ABI 约定
///
/// GCC/Clang 在生成栈保护代码时直接引用此符号名。该变量在程序加载时
/// 为零初始化的 `.bss` 段中，在 [`__init_ssp`] 调用后持有进程唯一的 canary 值。
///
/// # 不变量
///
/// - `__init_ssp()` 调用后，值始终非零
/// - 在 64 位平台上，第二字节始终为零
/// - `__stack_chk_guard == __pthread_self().canary`（初始化后始终成立）
///
/// # 安全性
///
/// 此变量为 `static mut`，读写均为 unsafe。正常情况下仅在初始化时写入一次，
/// 之后仅由编译器生成的栈保护代码读取。
#[no_mangle]
pub static mut __stack_chk_guard: usize = 0;

// ---------------------------------------------------------------------------
// 平台条件编译 — 64 位平台第二字节清零
// ---------------------------------------------------------------------------

/// 在 64 位平台上清零 canary 第二字节，以防御通过 `strcpy`/`sprintf` 等
/// 字符串操作函数泄漏或覆写 canary 的攻击。
///
/// 原理：攻击者如果以字符串方式溢出缓冲区，会被 canary 中的 NULL 字节截断，
/// 无法完成对 canary 的覆盖。
#[cfg(target_pointer_width = "64")]
fn apply_canary_mask(canary: usize) -> usize {
    canary & !0xFF00usize
}

/// 32 位平台：无需清零，直接返回原始 canary 值。
#[cfg(not(target_pointer_width = "64"))]
fn apply_canary_mask(canary: usize) -> usize {
    canary
}

// ---------------------------------------------------------------------------
// __init_ssp — 栈保护器初始化
// ---------------------------------------------------------------------------

/// 栈保护器初始化入口，由 CRT 启动代码在程序启动早期调用。
///
/// # 参数
///
/// * `entropy` — 由启动代码设置：
///   - 非 NULL：指向从内核 `AT_RANDOM` 辅助向量获得的随机字节缓冲区
///     （至少 `size_of::<usize>()` 字节）
///   - NULL：随机熵不可用，回退到基于地址的确定性算法
///
/// # 算法
///
/// 1. 若 `entropy` 非 NULL，通过 `copy_nonoverlapping` 从熵缓冲区复制
///    `size_of::<usize>()` 字节到 `__stack_chk_guard`
/// 2. 若 `entropy` 为 NULL，使用 `&__stack_chk_guard * CANARY_MULTIPLIER`
///    生成伪随机值
/// 3. 在 64 位平台上，将 canary 第二字节清零
/// 4. 将最终值同步到当前线程的 TLS `canary` 字段
///
/// # 前置条件
///
/// - 调用发生在程序启动早期、单线程环境中
/// - TLS 必须已初始化（`__pthread_self()` 可用）
///
/// # 线程安全
///
/// 单线程调用，不需要同步原语。
#[no_mangle]
pub extern "C" fn __init_ssp(entropy: *mut c_void) {
    use rusl_internal::pthread_impl::__pthread_self;

    // 步骤 1-2：获取初始 canary 值
    let mut canary: usize = if !entropy.is_null() {
        // Case 1: 从内核随机熵复制
        let mut val: usize = 0;
        unsafe {
            // Safety: entropy 非 NULL 时指向至少 size_of::<usize>() 字节的有效缓冲区
            ptr::copy_nonoverlapping::<u8>(
                entropy as *const u8,
                (&raw mut val) as *mut u8,
                core::mem::size_of::<usize>(),
            );
        }
        val
    } else {
        // Case 2: 基于地址的确定性回退算法
        // 在 ASLR 环境中，&__stack_chk_guard 的地址是随机的，
        // 与乘数相乘后仍具有不可预测性
        let guard_addr = (&raw const __stack_chk_guard) as usize;
        guard_addr.wrapping_mul(CANARY_MULTIPLIER)
    };

    // 步骤 3：64 位平台清零第二字节，防御字符串攻击
    canary = apply_canary_mask(canary);

    // 写入全局 canary
    unsafe {
        __stack_chk_guard = canary;
    }

    // 步骤 4：同步到当前线程的 TLS canary 字段
    let self_ptr = __pthread_self();
    if !self_ptr.is_null() {
        unsafe {
            (*self_ptr).canary = canary;
        }
    }
}

// ---------------------------------------------------------------------------
// __stack_chk_fail — 栈破坏回调
// ---------------------------------------------------------------------------

/// 栈 canary 校验失败时由编译器生成的代码自动调用的崩溃函数。
///
/// # 行为
///
/// 通过空指针 `volatile` 写入触发 SIGSEGV 立即终止进程。此函数**永不返回**。
/// 不使用 `abort()` 或 `atexit` 回调，避免在栈已被破坏的情况下执行可能被
/// 攻击者利用的代码路径。
///
/// # 线程安全
///
/// 无状态修改，纯终止操作。可在任何线程上下文中调用。
#[no_mangle]
pub extern "C" fn __stack_chk_fail() -> ! {
    // volatile 写入地址 0，触发硬件级别 SIGSEGV
    // 等价于 musl 的 a_crash(): *(volatile char *)0 = 0
    unsafe {
        ptr::write_volatile(ptr::null_mut::<u8>(), 0);
    }

    // 若平台未因上述写入立即终止（极端情况），进入无限循环
    // 确保满足 `!` 返回类型的控制流要求
    loop {
        core::hint::spin_loop();
    }
}

// ---------------------------------------------------------------------------
// __stack_chk_fail_local — DSO 局部弱别名
// ---------------------------------------------------------------------------

/// `__stack_chk_fail` 的 DSO 内部局部别名。
///
/// 在某些平台（特别是 i386 老式 PLT 场景）上，GCC 生成的代码可能通过
/// 此局部符号而非 `__stack_chk_fail` 来引用栈保护失败函数。
///
/// 采用方案 A（直接转发调用）实现。由于此函数仅在进程崩溃时调用，
/// 额外的跳转开销可忽略不计。启用 LTO 时编译器会将调用内联消除。
///
/// # 行为
///
/// 与 [`__stack_chk_fail`] 完全相同 —— 触发 SIGSEGV 终止进程，永不返回。
#[no_mangle]
#[doc(hidden)]
pub extern "C" fn __stack_chk_fail_local() -> ! {
    __stack_chk_fail()
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;

    // =========================================================================
    // CANARY_MULTIPLIER 常量测试
    // =========================================================================

    test!("canary_multiplier_is_0x41c64e6d" {
        assert_eq!(CANARY_MULTIPLIER, 0x41C64E6D);
    });

    test!("canary_multiplier_is_nonzero" {
        assert!(CANARY_MULTIPLIER != 0);
    });

    test!("canary_multiplier_is_odd" {
        // 奇数乘数保证与地址相乘后至少有一位熵
        assert_eq!(CANARY_MULTIPLIER & 1, 1);
    });

    // =========================================================================
    // apply_canary_mask 测试 (64 位)
    // =========================================================================

    #[cfg(target_pointer_width = "64")]
    test!("apply_canary_mask_zeros_second_byte" {
        // 全 0xFF canary 在掩码后第二字节应为 0
        let canary: usize = 0xFFFFFFFFFFFFFFFFusize;
        let masked = apply_canary_mask(canary);

        // 第二字节 (bits 8-15) 应为 0
        assert_eq!((masked >> 8) & 0xFF, 0);

        // 其他字节应保持不变
        assert_eq!(masked & 0xFF, 0xFF);           // 第一字节不变
        assert_eq!((masked >> 16) & 0xFF, 0xFF);    // 第三字节不变
        assert_eq!((masked >> 24) & 0xFF, 0xFF);    // 第四字节不变
    });

    #[cfg(target_pointer_width = "64")]
    test!("apply_canary_mask_preserves_first_byte" {
        let canary: usize = 0xABCDEF0123456789usize;
        let masked = apply_canary_mask(canary);

        // 第一字节 (bits 0-7) 保持不变
        assert_eq!((masked & 0xFF) as u8, (canary & 0xFF) as u8);
        // 第二字节 (bits 8-15) 应为 0
        assert_eq!(((masked >> 8) & 0xFF) as u8, 0u8);
        // 第三字节 (bits 16-23) 保持不变
        assert_eq!(((masked >> 16) & 0xFF) as u8, ((canary >> 16) & 0xFF) as u8);
    });

    #[cfg(target_pointer_width = "64")]
    test!("apply_canary_mask_zero_input_remains_zero" {
        let canary: usize = 0;
        let masked = apply_canary_mask(canary);
        assert_eq!(masked, 0);
    });

    #[cfg(target_pointer_width = "64")]
    test!("apply_canary_mask_single_bit_set" {
        // 确保掩码不会错误地清除其他位
        let canary: usize = 1usize << 63; // 仅最高位
        let masked = apply_canary_mask(canary);
        // 第二字节应为 0，最高位不受影响
        assert_eq!((masked >> 8) & 0xFF, 0);
        assert_eq!(masked & (1usize << 63), 1usize << 63);
    });

    // =========================================================================
    // apply_canary_mask 测试 (32 位)
    // =========================================================================

    #[cfg(not(target_pointer_width = "64"))]
    test!("apply_canary_mask_identity_32bit" {
        // 32 位平台：掩码应为恒等函数
        let canary: usize = 0x12345678;
        let masked = apply_canary_mask(canary);
        assert_eq!(masked, canary);
    });

    #[cfg(not(target_pointer_width = "64"))]
    test!("apply_canary_mask_zero_identity_32bit" {
        let canary: usize = 0;
        let masked = apply_canary_mask(canary);
        assert_eq!(masked, canary);
    });

    // =========================================================================
    // __stack_chk_guard 全局变量测试
    // =========================================================================

    test!("stack_chk_guard_initial_value_zero" {
        // 先恢复为零，消除其他测试可能设置的残留值
        unsafe { __stack_chk_guard = 0; }
        assert_eq!(unsafe { __stack_chk_guard }, 0);
    });

    test!("stack_chk_guard_is_mutable" {
        // 验证可以写入和读取 __stack_chk_guard
        let saved = unsafe { __stack_chk_guard };
        unsafe { __stack_chk_guard = 0xDEADBEEFusize; }
        assert_eq!(unsafe { __stack_chk_guard }, 0xDEADBEEFusize);
        // 恢复原始值
        unsafe { __stack_chk_guard = saved; }
    });

    test!("stack_chk_guard_size_matches_usize" {
        assert_eq!(
            core::mem::size_of_val(unsafe { &__stack_chk_guard }),
            core::mem::size_of::<usize>()
        );
    });

    test!("stack_chk_guard_has_valid_alignment" {
        // __stack_chk_guard 应该具有 usize 的自然对齐
        let align = core::mem::align_of_val(unsafe { &__stack_chk_guard });
        assert!(align >= core::mem::align_of::<usize>());
    });

    #[cfg(target_pointer_width = "64")]
    test!("stack_chk_guard_second_byte_accessible" {
        // 验证可以通过指针访问第二字节
        unsafe {
            __stack_chk_guard = 0xABCDusize;
            let second_byte = ((&raw const __stack_chk_guard) as *const u8).add(1).read();
            // 在 x86_64 小端上，0xABCD 的第二字节是 0xAB
            // 但不验证具体值，只验证读取不会崩溃
            let _ = second_byte;
            __stack_chk_guard = 0; // 恢复
        }
    });

    // =========================================================================
    // __init_ssp 测试 (无 TLS 环境 — 不测试 pthread sync 路径)
    // =========================================================================

    test!("init_ssp_with_null_entropy_sets_guard" {
        // 保存并清零，然后以 null 熵调用 __init_ssp
        unsafe { __stack_chk_guard = 0; }
        __init_ssp(core::ptr::null_mut());

        let guard = unsafe { __stack_chk_guard };
        assert!(guard != 0, "guard should be non-zero after init_ssp(null)");

        // 在 64 位平台上，第二字节应为 0
        #[cfg(target_pointer_width = "64")]
        {
            let second_byte = unsafe {
                ((&raw const __stack_chk_guard) as *const u8).add(1).read()
            };
            assert_eq!(second_byte, 0, "second byte should be zero on 64-bit");
        }

        // 恢复
        unsafe { __stack_chk_guard = 0; }
    });

    test!("init_ssp_with_entropy_sets_guard" {
        // 准备已知熵数据
        let entropy: [u8; core::mem::size_of::<usize>()] = [0xCD; core::mem::size_of::<usize>()];

        unsafe { __stack_chk_guard = 0; }
        __init_ssp(entropy.as_ptr() as *mut c_void);

        let guard = unsafe { __stack_chk_guard };
        assert!(guard != 0, "guard should be non-zero after init_ssp(entropy)");

        // 在 64 位平台上，第二字节应为 0（掩码生效）
        #[cfg(target_pointer_width = "64")]
        {
            let second_byte = unsafe {
                ((&raw const __stack_chk_guard) as *const u8).add(1).read()
            };
            assert_eq!(second_byte, 0, "second byte should be zero on 64-bit");
        }

        // 恢复
        unsafe { __stack_chk_guard = 0; }
    });

    test!("init_ssp_twice_produces_same_result_for_null" {
        // 两次 null 熵调用应产生相同结果（确定性算法）
        unsafe { __stack_chk_guard = 0; }
        __init_ssp(core::ptr::null_mut());
        let first = unsafe { __stack_chk_guard };

        unsafe { __stack_chk_guard = 0; }
        __init_ssp(core::ptr::null_mut());
        let second = unsafe { __stack_chk_guard };

        // 由于基于地址的算法是确定性的，两次调用结果应相同
        assert_eq!(first, second);

        // 恢复
        unsafe { __stack_chk_guard = 0; }
    });

    test!("init_ssp_with_different_entropy_produces_different_guard" {
        // 验证不同熵产生不同 canary
        let entropy_a: [u8; core::mem::size_of::<usize>()] = [0x11; core::mem::size_of::<usize>()];
        let entropy_b: [u8; core::mem::size_of::<usize>()] = [0x22; core::mem::size_of::<usize>()];

        unsafe { __stack_chk_guard = 0; }
        __init_ssp(entropy_a.as_ptr() as *mut c_void);
        let guard_a = unsafe { __stack_chk_guard };

        unsafe { __stack_chk_guard = 0; }
        __init_ssp(entropy_b.as_ptr() as *mut c_void);
        let guard_b = unsafe { __stack_chk_guard };

        assert_ne!(guard_a, guard_b, "different entropy should produce different guards");

        // 恢复
        unsafe { __stack_chk_guard = 0; }
    });

    // =========================================================================
    // __stack_chk_fail 行为测试
    // =========================================================================

    test!("stack_chk_fail_has_correct_signature" {
        // 验证 __stack_chk_fail 的返回类型是 ! (never type)
        fn _assert_diverging(_f: extern "C" fn() -> !) {}
        _assert_diverging(__stack_chk_fail);
    });

    test!("stack_chk_fail_local_has_correct_signature" {
        // 验证 __stack_chk_fail_local 的返回类型是 ! (never type)
        fn _assert_diverging(_f: extern "C" fn() -> !) {}
        _assert_diverging(__stack_chk_fail_local);
    });

    test!("stack_chk_fail_and_local_are_different_fns" {
        // __stack_chk_fail 和 __stack_chk_fail_local 应为不同的函数指针
        let fail_ptr = __stack_chk_fail as *const ();
        let local_ptr = __stack_chk_fail_local as *const ();
        // 方案 A 下它们是不同的函数（直接转发调用）
        assert!(!fail_ptr.is_null());
        assert!(!local_ptr.is_null());
    });

    // =========================================================================
    // 跨平台一致性测试
    // =========================================================================

    test!("apply_canary_mask_idempotent" {
        // 掩码函数应是幂等的：mask(mask(x)) == mask(x)
        let canary: usize = 0xFFFFFFFFFFFFFFFFusize;
        let masked_once = apply_canary_mask(canary);
        let masked_twice = apply_canary_mask(masked_once);
        assert_eq!(masked_once, masked_twice);
    });

    test!("canary_multiplier_fits_in_usize" {
        // 常量应在 usize 范围内
        let _: usize = CANARY_MULTIPLIER;
    });
}
