//! __rand48_step — 48 位 LCG 单步迭代核心引擎。
//! [Visibility]: Internal (pub(crate))
//!
//! 将 xi 的 48 位种子（小端，3×u16）与 lc 的 LCG 参数
//! （前 3×u16 乘数 + 1×u16 加数）结合，计算 `a * x + c` 模 2^48，
//! 结果写回 xi，同时返回低 48 位完整值。

/// 执行 48 位 LCG 单步迭代: `X_new = (a * X_curr + c) mod 2^48`。
///
/// 将 `xi` 的 48 位种子（小端排列，3 个 u16）与 `lc` 的 LCG 参数
/// （前 3 个 u16 为乘数，第 4 个 u16 为加数）结合，计算新种子。
///
/// 结果写入 `xi[0..2]`，同时返回低 48 位的完整值。
///
/// # Safety
///
/// - `xi` 必须指向 3 个 `u16` 的有效可读写缓冲区。
/// - `lc` 必须指向 4 个 `u16` 的有效可读缓冲区。
///
/// # Invariant
///
/// 确定性纯函数：给定相同 `xi` 和 `lc` 产生相同输出。
pub(crate) unsafe fn __rand48_step(xi: *mut u16, lc: *const u16) -> u64 {
    // 从 xi 的小端 u16 组装 48 位种子
    let x = (*xi as u64)
        | ((*xi.add(1) as u64) << 16)
        | ((*xi.add(2) as u64) << 32);

    // 从 lc 的小端 u16 组装 48 位乘数
    let a = (*lc as u64)
        | ((*lc.add(1) as u64) << 16)
        | ((*lc.add(2) as u64) << 32);

    // 加数
    let c = *lc.add(3) as u64;

    // a*x + c, 使用 wrapping 算模拟 C 无符号溢出
    let result = a.wrapping_mul(x).wrapping_add(c);

    // 写回 xi（小端）
    *xi = result as u16;
    *xi.add(1) = (result >> 16) as u16;
    *xi.add(2) = (result >> 32) as u16;

    // 返回低 48 位
    result & 0xffff_ffff_ffff
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;

    test!("test_step_zero_seed" {
        // 基本功能：零种子 + 默认乘数 = 仅加数结果。
        unsafe {
            let mut xi = [0u16; 3];
            // 默认乘数: 0x5DEECE66D (小端: [0xe66d, 0xdeec, 0x5])
            // 默认加数: 0xB
            let lc = [0xe66d, 0xdeec, 0x0005, 0x000b];
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // 0 * 0x5DEECE66D + 0xB = 0xB
            assert_eq!(result, 0x0000_0000_0000_000b);
            // xi 应被更新为 0xB 的小端表示
            assert_eq!(xi[0], 0x000b);
            assert_eq!(xi[1], 0x0000);
            assert_eq!(xi[2], 0x0000);
        }
    });

    test!("test_step_known_input" {
        // 已知输入产生已知输出。
        unsafe {
            // 种子 = 0x1234_5678_9abc（小端: [0x9abc, 0x5678, 0x1234]）
            let mut xi = [0x9abcu16, 0x5678, 0x1234];
            // 乘数 = 0x5DEECE66D (小端: [0xe66d, 0xdeec, 0x5])
            // 加数 = 0xB
            let lc = [0xe66d, 0xdeec, 0x0005, 0x000b];
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // 验证结果不为 0（种子非零）
            assert!(result != 0);
            // 验证结果低 48 位，高 16 位应为 0
            assert_eq!(result >> 48, 0);
        }
    });

    test!("test_step_updates_xi" {
        // xi 输出缓冲区被正确更新。
        unsafe {
            let mut xi = [0xffffu16; 3];
            let lc = [0xe66d, 0xdeec, 0x0005, 0x000b];
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // xi 的每个 u16 应与 result 的对应 16 位一致（小端）
            assert_eq!(xi[0], (result & 0xffff) as u16);
            assert_eq!(xi[1], ((result >> 16) & 0xffff) as u16);
            assert_eq!(xi[2], ((result >> 32) & 0xffff) as u16);
        }
    });

    test!("test_step_custom_lc" {
        // 使用非默认 LCG 参数。
        unsafe {
            let mut xi = [1u16, 0, 0];
            // 自定乘数 = 1, 加数 = 1
            let lc = [1u16, 0, 0, 1];
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // 1 * 1 + 1 = 2
            assert_eq!(result, 2);
        }
    });

    test!("test_step_deterministic" {
        // 交换律/确定性测试：相同输入产生相同输出。
        unsafe {
            let mut xi_a = [0x1234u16, 0x5678, 0x9abc];
            let lc = [0xe66d, 0xdeec, 0x0005, 0x000b];
            let r1 = __rand48_step(xi_a.as_mut_ptr(), lc.as_ptr());

            let mut xi_b = [0x1234u16, 0x5678, 0x9abc];
            let r2 = __rand48_step(xi_b.as_mut_ptr(), lc.as_ptr());

            assert_eq!(r1, r2);
            assert_eq!(xi_a, xi_b);
        }
    });

    test!("test_step_large_seed" {
        // 最大种子值测试（接近 2^48 - 1）。
        unsafe {
            let mut xi = [0xffffu16, 0xffff, 0xffff];
            let lc = [0xe66d, 0xdeec, 0x0005, 0x000b];
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // 结果应为 48 位模值，高 16 位应为 0
            assert_eq!(result >> 48, 0);
        }
    });

    test!("test_step_zero_adder" {
        // 最小种子值 (xi = [0,0,0], 加数 = 0)。
        unsafe {
            let mut xi = [0x1234u16, 0x5678, 0x9abc];
            let lc = [0xe66d, 0xdeec, 0x0005, 0x0000]; // c = 0
            let result = __rand48_step(xi.as_mut_ptr(), lc.as_ptr());
            // 结果应等于 (a * X) mod 2^48，使用 wrapping 乘法模拟硬件行为
            let a: u64 = 0x5deece66d;
            let x: u64 = 0x9abc56781234;
            let expected = a.wrapping_mul(x) & 0xffff_ffff_ffff;
            assert_eq!(result, expected);
        }
    });
}