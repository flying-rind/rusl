//! __seed48 — 48 位 LCG 的全局种子和参数存储。
//! [Visibility]: Internal (pub(crate))
//!
//! 数组布局：
//! - `[0..2]`: 48 位种子 (小端排列)
//! - `[3..5]`: 默认乘数 (小端排列，对应 0x5DEECE66D)
//! - `[6]`   : 默认加数 (对应 0xB)
//!
//! 默认初始化为标准 48 位 LCG 参数：
//! - 种子: {0, 0, 0}
//! - 乘数: {0xe66d, 0xdeec, 0x5} (0x5DEECE66D)
//! - 加数: {0xb} (0xB)

/// 48 位 LCG 的全局种子和参数。
///
/// # Safety
///
/// 全局可变状态，非线程安全。C 中通过 TLS 访问，Rust 中使用 `static mut`。
#[allow(non_upper_case_globals)]
pub static mut __seed48: [u16; 7] = [
    0,      // __seed48[0]: 种子低 16 位
    0,      // __seed48[1]: 种子中 16 位
    0,      // __seed48[2]: 种子高 16 位
    0xe66d, // __seed48[3]: 乘数低 16 位
    0xdeec, // __seed48[4]: 乘数中 16 位
    0x0005, // __seed48[5]: 乘数高 16 位
    0x000b, // __seed48[6]: 加数
];

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    test!("test_initial_values" {
        // 验证初始值与 spec 一致。
        unsafe {
            assert_eq!(__seed48[0], 0);
            assert_eq!(__seed48[1], 0);
            assert_eq!(__seed48[2], 0);
            assert_eq!(__seed48[3], 0xe66d);
            assert_eq!(__seed48[4], 0xdeec);
            assert_eq!(__seed48[5], 0x5);
            assert_eq!(__seed48[6], 0xb);
        }
    });

    test!("test_array_length" {
        // 验证数组长度正确 (7 个 u16 构成完整状态)。
        unsafe {
            assert_eq!(__seed48.len(), 7);
        }
    });

    test!("test_default_multiplier_value" {
        // 验证默认乘数组合值：0x5DEECE66D。
        unsafe {
            let m: u64 =
                (__seed48[3] as u64) | ((__seed48[4] as u64) << 16) | ((__seed48[5] as u64) << 32);
            assert_eq!(m, 0x5DEECE66D);
        }
    });

    test!("test_default_adder_value" {
        // 验证默认加数值。
        unsafe {
            assert_eq!(__seed48[6], 0xB);
        }
    });

    test!("test_can_modify_seed" {
        // 验证可以修改种子字段。
        unsafe {
            __seed48[0] = 0x1234;
            __seed48[1] = 0x5678;
            __seed48[2] = 0x9abc;
            assert_eq!(__seed48[0], 0x1234);
            assert_eq!(__seed48[1], 0x5678);
            assert_eq!(__seed48[2], 0x9abc);
            // 恢复原始值
            __seed48[0] = 0;
            __seed48[1] = 0;
            __seed48[2] = 0;
        }
    });
}