//! complex_impl 模块 — 复数类型定义与构造辅助。
//!
//! 本模块定义了 rusl 数学库所需的内部复数类型 `Complex32`（对应
//! `float complex`）和 `Complex64`（对应 `double complex`），
//! 以及复数构造宏 `CMPLXF`/`CMPLX` 和两个内部复数函数声明。
//!
//! 在 x86_64 上，`long double complex` 对应 80 位 x87 扩展精度，
//! 定义为 `Complex80` 类型。
//!
//! # 数学函数说明
//!
//! `exp`/`cos`/`sin` 等超越函数在 Rust `core` 中不可用（仅 `std` 提供）。
//! 本模块使用泰勒级数 + 位操作实现这些函数，无需任何外部库依赖，
//! 完全兼容 `#![no_std]` 环境。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// 复数类型定义
// ---------------------------------------------------------------------------

/// 单精度复数类型（对应 C 的 `float complex`）。
///
/// `#[repr(C)]` 确保实部在前、虚部在后，与 C ABI 兼容。
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Complex32 {
    /// 实部
    pub re: f32,
    /// 虚部
    pub im: f32,
}

/// 双精度复数类型（对应 C 的 `double complex`）。
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Complex64 {
    /// 实部
    pub re: f64,
    /// 虚部
    pub im: f64,
}

/// 80 位扩展精度复数类型（x86_64 上的 `long double complex`）。
///
/// 注意：Rust 不原生支持 `f80` 类型，此类型使用字节数组作为占位。
/// 实际实现可能需要自定义浮点运算或使用内联 x87 汇编。
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Complex80 {
    /// 实部（包含 6 字节尾部填充以保持 16 字节对齐）
    pub re: [u8; 16],
    /// 虚部
    pub im: [u8; 16],
}

// ---------------------------------------------------------------------------
// 复数构造方法
// ---------------------------------------------------------------------------

impl Complex32 {
    /// 用两个标量值构造单精度复数。
    ///
    /// 等价于 C 的 `CMPLXF(x, y)` 宏。
    #[inline]
    pub fn new(re: f32, im: f32) -> Self {
        Complex32 { re, im }
    }
}

impl Complex64 {
    /// 用两个标量值构造双精度复数。
    ///
    /// 等价于 C 的 `CMPLX(x, y)` 宏。
    #[inline]
    pub fn new(re: f64, im: f64) -> Self {
        Complex64 { re, im }
    }
}

impl Complex80 {
    /// 用两个标量值构造扩展精度复数。
    ///
    /// 等价于 C 的 `CMPLXL(x, y)` 宏（x86_64 平台）。
    #[inline]
    pub fn new(_re: [u8; 16], _im: [u8; 16]) -> Self {
        Complex80 { re: _re, im: _im }
    }
}

// ---------------------------------------------------------------------------
// 复数构造宏 (保持与 C musl 代码相同的调用风格)
// ---------------------------------------------------------------------------

/// 构造单精度复数：`CMPLXF(x, y)` → `Complex32::new(x as f32, y as f32)`
#[macro_export]
macro_rules! CMPLXF {
    ($x:expr, $y:expr) => {
        $crate::complex_impl::Complex32::new($x as f32, $y as f32)
    };
}

/// 构造双精度复数：`CMPLX(x, y)` → `Complex64::new(x as f64, y as f64)`
#[macro_export]
macro_rules! CMPLX {
    ($x:expr, $y:expr) => {
        $crate::complex_impl::Complex64::new($x as f64, $y as f64)
    };
}

// =========================================================================
// 内部 no_std 数学函数
// =========================================================================
// 以下函数使用纯位操作和泰勒级数实现,不依赖 libm 或 std。
// 所有实现均为私有辅助函数,仅供本模块内部使用。

/// 计算 2^n（f64, 纯位操作）。
///
/// 直接构造 IEEE 754 双精度浮点数:
///   2^n = 1.0 * 2^n = from_bits((biased_exponent) << 52)
#[inline]
fn pow2_f64(n: i32) -> f64 {
    if n < -1074 {
        return 0.0; // 下溢至零
    }
    if n > 1023 {
        return f64::INFINITY; // 上溢
    }
    f64::from_bits(((n + 1023) as u64) << 52)
}

/// 计算 2^n（f32, 纯位操作）。
#[inline]
fn pow2_f32(n: i32) -> f32 {
    if n < -149 {
        return 0.0;
    }
    if n > 127 {
        return f32::INFINITY;
    }
    f32::from_bits(((n + 127) as u32) << 23)
}

/// floor(x) 实现 — 无 libm 依赖。
///
/// 使用 `as i64` 截断 + 负值修正。仅对有限数安全使用;
/// NaN/Inf 由调用方保护。
#[inline]
fn floor_f64(x: f64) -> f64 {
    let i = x as i64;
    let fi = i as f64;
    if x < fi {
        fi - 1.0
    } else {
        fi
    }
}

/// floor(x) — f32 版本。
#[inline]
fn floor_f32(x: f32) -> f32 {
    let i = x as i32;
    let fi = i as f32;
    if x < fi {
        fi - 1.0
    } else {
        fi
    }
}

/// exp(x) 泰勒级数实现（f64, no_std）。
///
/// 算法:
///   1. 范围缩减: x = k*ln(2) + r, |r| <= ln(2)/2
///   2. Taylor 级数求 exp(r) (12 项, Horner 格式)
///   3. exp(x) = 2^k * exp(r)
fn exp_f64(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    // 溢出阈值: ln(f64::MAX) ≈ 709.7827
    if x > 709.782712893384 {
        return f64::INFINITY;
    }
    // 下溢阈值: ln(最小正规数) ≈ -744.44
    if x < -745.1332191019411 {
        return 0.0;
    }

    // 范围缩减: k ≈ round(x / ln(2)), r = x - k*ln(2)
    let inv_ln2: f64 = 1.4426950408889634; // 1 / ln(2)
    let k = floor_f64(x * inv_ln2 + 0.5);
    let ki = k as i32;
    let r = x - k * core::f64::consts::LN_2;

    // 对 |r| < ln(2)/2 ≈ 0.3466 计算 Taylor 级数
    // exp(r) = 1 + r/1! + r²/2! + ... + r¹¹/11!
    // Horner 格式: 从高次项开始累加
    let mut s = 1.0 + r / 12.0;
    s = 1.0 + r * s / 11.0;
    s = 1.0 + r * s / 10.0;
    s = 1.0 + r * s / 9.0;
    s = 1.0 + r * s / 8.0;
    s = 1.0 + r * s / 7.0;
    s = 1.0 + r * s / 6.0;
    s = 1.0 + r * s / 5.0;
    s = 1.0 + r * s / 4.0;
    s = 1.0 + r * s / 3.0;
    s = 1.0 + r * s / 2.0;
    s = 1.0 + r * s;

    s * pow2_f64(ki)
}

/// sin(x) 泰勒级数实现（f64, no_std）。
///
/// 算法:
///   1. 范围缩减到 [-PI/2, PI/2] 象限
///   2. Taylor 级数求 sin(r) (8 项, |r| < PI/2)
fn sin_f64(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }

    let half_pi = core::f64::consts::FRAC_PI_2;

    // 范围缩减: k = round(x / (pi/2)), r = x - k*(pi/2)
    let k = floor_f64(x / half_pi + 0.5);
    let ki = (k as i32) & 3; // 象限编号 0..3
    let r = x - k * half_pi;

    // Taylor 级数: sin(r) = r - r³/3! + r⁵/5! - r⁷/7! + r⁹/9! - r¹¹/11! + r¹³/13! - r¹⁵/15!
    // Horner 格式: sin(r) = r * P(r²)
    let r2 = r * r;
    let p = 1.0_f64
        + r2 * (-1.0 / 6.0
        + r2 * (1.0 / 120.0
        + r2 * (-1.0 / 5040.0
        + r2 * (1.0 / 362880.0
        + r2 * (-1.0 / 39916800.0
        + r2 * (1.0 / 6227020800.0
        + r2 * (-1.0 / 1307674368000.0)))))));
    let s = r * p;

    match ki {
        0 => s,               // sin(x) = sin(r)
        1 => cos_taylor_f64(r), // sin(x) = cos(r)
        2 => -s,               // sin(x) = -sin(r)
        _ => -cos_taylor_f64(r), // sin(x) = -cos(r)  (ki==3)
    }
}

/// cos(x) 通过恒等式 cos(x) = sin(PI/2 - x)
fn cos_f64(x: f64) -> f64 {
    sin_f64(core::f64::consts::FRAC_PI_2 - x)
}

/// cos(r) 的 Taylor 级数 (|r| < PI/2), 8 项。
///
/// cos(r) = 1 - r²/2! + r⁴/4! - r⁶/6! + r⁸/8! - r¹⁰/10! + r¹²/12! - r¹⁴/14!
fn cos_taylor_f64(r: f64) -> f64 {
    let r2 = r * r;
    1.0_f64
        + r2 * (-1.0 / 2.0
        + r2 * (1.0 / 24.0
        + r2 * (-1.0 / 720.0
        + r2 * (1.0 / 40320.0
        + r2 * (-1.0 / 3628800.0
        + r2 * (1.0 / 479001600.0
        + r2 * (-1.0 / 87178291200.0)))))))
}

// --- f32 版本 ---

/// exp(x) — f32 版本 (8 项 Taylor).
fn exp_f32(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x > 88.722839_f32 {
        return f32::INFINITY;
    }
    if x < -87.336544_f32 {
        return 0.0;
    }

    let inv_ln2: f32 = 1.4426950408889634_f32;
    let k = floor_f32(x * inv_ln2 + 0.5_f32);
    let ki = k as i32;
    let r = x - k * core::f32::consts::LN_2;

    // Horner, 8 项
    let mut s = 1.0_f32 + r / 8.0_f32;
    s = 1.0_f32 + r * s / 7.0_f32;
    s = 1.0_f32 + r * s / 6.0_f32;
    s = 1.0_f32 + r * s / 5.0_f32;
    s = 1.0_f32 + r * s / 4.0_f32;
    s = 1.0_f32 + r * s / 3.0_f32;
    s = 1.0_f32 + r * s / 2.0_f32;
    s = 1.0_f32 + r * s;

    s * pow2_f32(ki)
}

/// sin(x) — f32 版本 (6 项 Taylor).
fn sin_f32(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }

    let half_pi = core::f32::consts::FRAC_PI_2;
    let k = floor_f32(x / half_pi + 0.5_f32);
    let ki = (k as i32) & 3;
    let r = x - k * half_pi;

    // Taylor: sin(r) = r - r³/3! + r⁵/5! - r⁷/7! + r⁹/9! - r¹¹/11!
    let r2 = r * r;
    let p = 1.0_f32
        + r2 * (-1.0_f32 / 6.0_f32
        + r2 * (1.0_f32 / 120.0_f32
        + r2 * (-1.0_f32 / 5040.0_f32
        + r2 * (1.0_f32 / 362880.0_f32
        + r2 * (-1.0_f32 / 39916800.0_f32)))));
    let s = r * p;

    match ki {
        0 => s,
        1 => cos_taylor_f32(r),
        2 => -s,
        _ => -cos_taylor_f32(r),
    }
}

/// cos(x) — f32 版本.
fn cos_f32(x: f32) -> f32 {
    sin_f32(core::f32::consts::FRAC_PI_2 - x)
}

/// cos(r) Taylor — f32 (6 项).
fn cos_taylor_f32(r: f32) -> f32 {
    let r2 = r * r;
    1.0_f32
        + r2 * (-1.0_f32 / 2.0_f32
        + r2 * (1.0_f32 / 24.0_f32
        + r2 * (-1.0_f32 / 720.0_f32
        + r2 * (1.0_f32 / 40320.0_f32
        + r2 * (-1.0_f32 / 3628800.0_f32)))))
}

// =========================================================================
// 公开的复数数学函数
// =========================================================================

/// 计算 `ldexp(cexp(z), n)` 的融合操作（双精度版本）。
///
/// 算法:
///   cexp(z) = exp(re) * (cos(im) + i*sin(im))
///   __ldexp_cexp(z, n) = cexp(z) * 2^n
///
/// 使用本模块内部的 `exp_f64`/`sin_f64`/`cos_f64` 和位操作 `pow2_f64`,
/// 完全兼容 `no_std` 环境,无需 libm。
///
/// # 参数
///
/// * `z` - 输入复数（双精度）
/// * `n` - 整数指数
///
/// # 返回值
///
/// `exp(z) * 2^n` 的复数结果
pub fn __ldexp_cexp(z: Complex64, n: c_int) -> Complex64 {
    let exp_re = exp_f64(z.re);
    let cos_im = cos_f64(z.im);
    let sin_im = sin_f64(z.im);
    // ldexp factor: 2^n, 使用位构造避免 libm 依赖
    let factor = pow2_f64(n as i32);
    Complex64 {
        re: exp_re * cos_im * factor,
        im: exp_re * sin_im * factor,
    }
}

/// 计算 `ldexpf(cexpf(z), n)` 的融合操作（单精度版本）。
///
/// # 参数
///
/// * `z` - 输入复数（单精度）
/// * `n` - 整数指数
///
/// # 返回值
///
/// `exp(z) * 2^n` 的复数结果
pub fn __ldexp_cexpf(z: Complex32, n: c_int) -> Complex32 {
    let exp_re = exp_f32(z.re);
    let cos_im = cos_f32(z.im);
    let sin_im = sin_f32(z.im);
    let factor = pow2_f32(n as i32);
    Complex32 {
        re: exp_re * cos_im * factor,
        im: exp_re * sin_im * factor,
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::{Complex32, Complex64, Complex80};
    use crate::complex_impl::{__ldexp_cexp, __ldexp_cexpf};

    test!("complex32_size" {
        assert_eq!(core::mem::size_of::<Complex32>(), 8);
    });

    test!("complex64_size" {
        assert_eq!(core::mem::size_of::<Complex64>(), 16);
    });

    test!("complex80_size" {
        assert_eq!(core::mem::size_of::<Complex80>(), 32);
    });

    test!("complex32_copy" {
        let z = Complex32 { re: 1.0, im: 2.0 };
        let z2 = z;
        assert_eq!(z, z2);
    });

    test!("complex64_debug" {
        let z = Complex64 { re: 3.0, im: 4.0 };
        let _ = z;
        assert_eq!(z.re, 3.0);
        assert_eq!(z.im, 4.0);
    });

    // -----------------------------------------------------------------------
    // 构造函数测试
    // -----------------------------------------------------------------------

    test!("complex32_new" {
        let z = Complex32::new(1.5_f32, -2.5_f32);
        assert_eq!(z.re, 1.5_f32);
        assert_eq!(z.im, -2.5_f32);
    });

    test!("complex64_new" {
        let z = Complex64::new(3.14159_f64, 2.71828_f64);
        assert_eq!(z.re, 3.14159_f64);
        assert_eq!(z.im, 2.71828_f64);
    });

    test!("complex80_new" {
        let re = [1_u8; 16];
        let im = [2_u8; 16];
        let z = Complex80::new(re, im);
        assert_eq!(z.re, [1_u8; 16]);
        assert_eq!(z.im, [2_u8; 16]);
    });

    // -----------------------------------------------------------------------
    // __ldexp_cexp 测试 (f64)
    // -----------------------------------------------------------------------

    test!("ldexp_cexp_trivial" {
        // z = (0, 0), n = 0: cexp(0) = 1, ldexp(1, 0) = 1
        let z = Complex64::new(0.0, 0.0);
        let res = __ldexp_cexp(z, 0);
        assert!((res.re - 1.0).abs() < 1e-12);
        assert!((res.im).abs() < 1e-12);
    });

    test!("ldexp_cexp_n_zero" {
        // z = (0,0), n = 0: cexp(0) = 1
        let z = Complex64::new(0.0, 0.0);
        let res = __ldexp_cexp(z, 0);
        assert!((res.re - 1.0).abs() < 1e-12);
        assert!((res.im).abs() < 1e-12);
    });

    test!("ldexp_cexp_n_positive" {
        // z = ln(2), n = 3: exp(ln(2)) * 2^3 = 2 * 8 = 16
        let z = Complex64::new(core::f64::consts::LN_2, 0.0);
        let res = __ldexp_cexp(z, 3);
        assert!((res.re - 16.0).abs() < 1e-10);
        assert!((res.im).abs() < 1e-10);
    });

    test!("ldexp_cexp_n_negative" {
        // z = ln(8), n = -2: exp(ln(8)) * 2^(-2) = 8 * 0.25 = 2.0
        let z = Complex64::new(core::f64::consts::LN_2 * 3.0, 0.0);
        let res = __ldexp_cexp(z, -2);
        assert!((res.re - 2.0).abs() < 1e-10);
        assert!((res.im).abs() < 1e-10);
    });

    test!("ldexp_cexp_imag_nonzero" {
        // z = (0, pi/2): cexp(z) = exp(0)*(cos(pi/2) + i*sin(pi/2)) = 0 + i
        // ldexp(i, 2) = i * 4 = 4i
        let z = Complex64::new(0.0, core::f64::consts::FRAC_PI_2);
        let res = __ldexp_cexp(z, 2);
        assert!((res.re).abs() < 1e-10);
        assert!((res.im - 4.0).abs() < 1e-10);
    });

    test!("ldexp_cexp_both_nonzero" {
        // z = (ln(2), pi/2): exp(ln(2)) * i = 2i, ldexp(2i, 1) = 4i
        let z = Complex64::new(core::f64::consts::LN_2, core::f64::consts::FRAC_PI_2);
        let res = __ldexp_cexp(z, 1);
        // exp(LN_2) = 2, cos(pi/2)=0, sin(pi/2)=1 → (0, 2) → *2^1 → (0, 4)
        assert!((res.re).abs() < 1e-10);
        assert!((res.im - 4.0).abs() < 1e-10);
    });

    // -----------------------------------------------------------------------
    // __ldexp_cexpf 测试 (f32)
    // -----------------------------------------------------------------------

    test!("ldexp_cexpf_trivial" {
        let z = Complex32::new(0.0_f32, 0.0_f32);
        let res = __ldexp_cexpf(z, 0);
        assert!((res.re - 1.0_f32).abs() < 1e-5);
        assert!((res.im).abs() < 1e-5);
    });

    test!("ldexp_cexpf_n_positive" {
        // z = ln(2), n = 3: exp(ln(2)) * 2^3 = 16
        let z = Complex32::new(core::f32::consts::LN_2, 0.0_f32);
        let res = __ldexp_cexpf(z, 3);
        assert!((res.re - 16.0_f32).abs() < 1e-3);
        assert!((res.im).abs() < 1e-3);
    });

    test!("ldexp_cexpf_n_negative" {
        // z = ln(8), n = -2: 8 * 0.25 = 2.0
        let z = Complex32::new(core::f32::consts::LN_2 * 3.0_f32, 0.0_f32);
        let res = __ldexp_cexpf(z, -2);
        assert!((res.re - 2.0_f32).abs() < 1e-3);
        assert!((res.im).abs() < 1e-3);
    });

    test!("ldexp_cexpf_imag_nonzero" {
        // z = (0, pi/2): cexp(z) = i; ldexp(i, 2) = 4i
        let z = Complex32::new(0.0_f32, core::f32::consts::FRAC_PI_2);
        let res = __ldexp_cexpf(z, 2);
        assert!((res.re).abs() < 1e-3);
        assert!((res.im - 4.0_f32).abs() < 1e-3);
    });

    // -----------------------------------------------------------------------
    // 内部数学函数 — 精度测试
    // -----------------------------------------------------------------------

    test!("exp_f64_accuracy" {
        // exp(0) = 1
        let z = Complex64::new(0.0, 0.0);
        let res = __ldexp_cexp(z, 0);
        assert!((res.re - 1.0).abs() < 1e-12);
        assert!((res.im - 0.0).abs() < 1e-12);
    });

    test!("sin_f64_pi_half" {
        // sin(pi/2) = 1 → cexp(0 + i*pi/2) = i
        let z = Complex64::new(0.0, core::f64::consts::FRAC_PI_2);
        let res = __ldexp_cexp(z, 0);
        assert!((res.re).abs() < 1e-10);
        assert!((res.im - 1.0).abs() < 1e-10);
    });

    test!("cos_f64_pi_half" {
        // cos(pi/2) = 0
        let z = Complex64::new(0.0, core::f64::consts::FRAC_PI_2);
        let res = __ldexp_cexp(z, 0);
        assert!((res.re).abs() < 1e-10);
    });

    test!("exp_f32_accuracy" {
        let z = Complex32::new(0.0_f32, 0.0_f32);
        let res = __ldexp_cexpf(z, 0);
        assert!((res.re - 1.0_f32).abs() < 1e-4);
    });
}