/// 模块: div_test
/// `div` 集成测试

use super::imports::{div, ldiv, lldiv, imaxdiv};
use test_framework::test;

// ---- div 测试 ----

test!("test_div_positive" {
    // 测试 div：正数除法。
    {
        let r = div(10, 3);
        assert_eq!(r.quot, 3);
        assert_eq!(r.rem, 1);
    }
});

test!("test_div_negative_numerator" {
    // 测试 div：负数除以正数。
    {
        let r = div(-10, 3);
        assert_eq!(r.quot, -3);
        assert_eq!(r.rem, -1);
    }
});

test!("test_div_negative_denominator" {
    // 测试 div：正数除以负数。
    {
        let r = div(10, -3);
        assert_eq!(r.quot, -3);
        assert_eq!(r.rem, 1);
    }
});

test!("test_div_both_negative" {
    // 测试 div：两个负数。
    {
        let r = div(-10, -3);
        assert_eq!(r.quot, 3);
        assert_eq!(r.rem, -1);
    }
});

test!("test_div_zero_numerator" {
    // 测试 div：被除数为 0。
    {
        let r = div(0, 42);
        assert_eq!(r.quot, 0);
        assert_eq!(r.rem, 0);
    }
});

test!("test_div_denominator_one" {
    // 测试 div：除以 1。
    {
        let r = div(42, 1);
        assert_eq!(r.quot, 42);
        assert_eq!(r.rem, 0);
    }
});

test!("test_div_denominator_negative_one" {
    // 测试 div：除以 -1。
    {
        let r = div(42, -1);
        assert_eq!(r.quot, -42);
        assert_eq!(r.rem, 0);
    }
});

test!("test_div_max_values" {
    // 测试 div：最大/最小值。
    {
        let r = div(i32::MAX, 1);
        assert_eq!(r.quot, i32::MAX);
        assert_eq!(r.rem, 0);
    }
});

// ---- ldiv 测试 ----

test!("test_ldiv_basic" {
    // 测试 ldiv 基本功能。
    {
        let r = ldiv(100, 30);
        assert_eq!(r.quot, 3);
        assert_eq!(r.rem, 10);
    }
});

test!("test_ldiv_negative" {
    // 测试 ldiv 负数。
    {
        let r = ldiv(-100, 30);
        assert_eq!(r.quot, -3);
        assert_eq!(r.rem, -10);
    }
});

test!("test_ldiv_large" {
    // 测试 ldiv 大值。
    {
        let r = ldiv(1_000_000_000_000, 3);
        assert_eq!(r.quot, 333_333_333_333);
        assert_eq!(r.rem, 1);
    }
});

test!("test_ldiv_max" {
    // 测试 ldiv i64::MAX。
    {
        let r = ldiv(i64::MAX, 2);
        // i64::MAX / 2 = 4611686018427387903, i64::MAX % 2 = 1
        assert_eq!(r.quot, 4611686018427387903);
        assert_eq!(r.rem, 1);
    }
});

// ---- lldiv 测试 ----

test!("test_lldiv_basic" {
    // 测试 lldiv 基本功能（与 ldiv 行为相同）。
    {
        let r = lldiv(50, 7);
        assert_eq!(r.quot, 7);
        assert_eq!(r.rem, 1);
    }
});

test!("test_lldiv_negative" {
    // 测试 lldiv 负数。
    {
        let r = lldiv(-50, 7);
        assert_eq!(r.quot, -7);
        assert_eq!(r.rem, -1);
    }
});

test!("test_lldiv_exact" {
    // 测试 lldiv 完全整除。
    {
        let r = lldiv(100, 5);
        assert_eq!(r.quot, 20);
        assert_eq!(r.rem, 0);
    }
});

// ---- imaxdiv 测试 ----

test!("test_imaxdiv_basic" {
    // 测试 imaxdiv 基本功能（与 div 行为相同，类型为 i64）。
    {
        let r = imaxdiv(100, 30);
        assert_eq!(r.quot, 3);
        assert_eq!(r.rem, 10);
    }
});

test!("test_imaxdiv_negative" {
    // 测试 imaxdiv 负数。
    {
        let r = imaxdiv(-100, 30);
        assert_eq!(r.quot, -3);
        assert_eq!(r.rem, -10);
    }
});

test!("test_imaxdiv_edge" {
    // 测试 imaxdiv 边界（TMIN / -1 为 UB，这里仅测试其他边界）。
    {
        let r = imaxdiv(i64::MAX, -1);
        assert_eq!(r.quot, -i64::MAX);
        assert_eq!(r.rem, 0);
    }
});

// ---- 验证 num == quot * den + rem ----

test!("test_div_invariant" {
    // 验证 div 的不变量：num == quot * den + rem。
    {
        let cases = [(10, 3), (-10, 3), (10, -3), (-10, -3), (0, 5), (i32::MAX, 7)];
        for &(num, den) in &cases {
            if den == 0 { continue; }
            let r = div(num, den);
            assert_eq!(num, r.quot * den + r.rem, "不变量被违反: num={}, den={}", num, den);
        }
    }
});

test!("test_ldiv_invariant" {
    // 验证 ldiv 的不变量：num == quot * den + rem。
    {
        let cases = [(100i64, 30), (-100, 30), (100, -30), (-100, -30), (0, 5)];
        for &(num, den) in &cases {
            if den == 0 { continue; }
            let r = ldiv(num, den);
            assert_eq!(num, r.quot * den + r.rem, "不变量被违反: num={}, den={}", num, den);
        }
    }
});
