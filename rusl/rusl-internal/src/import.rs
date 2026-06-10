//! 声明所有依赖其他模块的接口
//!
//! 当不开启rusl feature时，使用musl的C接口
//!
//! 此处声明的 extern "C" 接口来自 musl 的非 internal 模块
//! （如 stdio/ctype），这些符号在最终链接时由 musl 提供。

#[cfg(not(feature = "rusl"))]
mod errno {
    use core::ffi::c_int;
    extern "C" {
        #[link_name = "__errno_location"]
        fn musl_errno_location() -> *mut c_int;
    }

    pub extern "C" fn __errno_location() -> *mut c_int {
        unsafe { musl_errno_location() }
    }
}

/// stdio 子系统提供的接口（musl src/stdio/）
#[cfg(not(feature = "rusl"))]
mod stdio {
    use crate::file::FILE;
    use core::ffi::c_int;

    extern "C" {
        /// 从 FILE 流中获取下一个字符（必要时填充缓冲区）
        /// 定义: musl src/stdio/__uflow.c
        pub fn __uflow(f: *mut FILE) -> c_int;

        /// 将 FILE 切换到读模式
        /// 定义: musl src/stdio/__toread.c
        pub fn __toread(f: *mut FILE) -> c_int;
    }
}

/// floatscan 辅助函数（由 musl floatscan_wrap.c 提供 80-bit 中间精度）
#[cfg(not(feature = "rusl"))]
mod floatscan_helpers {
    extern "C" {
        pub fn __floatscan_scale(y: f64, e2: i32) -> f64;
        pub fn __floatscan_mul(a: f64, b: f64) -> f64;
        pub fn __floatscan_abs(x: f64) -> f64;
        pub fn __floatscan_copysign(x: f64, y: f64) -> f64;
        pub fn __floatscan_fmod(x: f64, y: f64) -> f64;
    }
}

// ---------------------------------------------------------------------------
// rusl 自身实现（feature = "rusl"）
// ---------------------------------------------------------------------------

/// rusl-stdio 提供的 __uflow / __toread（#[no_mangle] 符号，链接时解析）
#[cfg(feature = "rusl")]
mod stdio {
    use crate::file::FILE;
    use core::ffi::c_int;

    extern "C" {
        /// 由 rusl-stdio 提供 #[no_mangle] 实现
        /// 定义: rusl-stdio/src/__uflow.rs
        pub fn __uflow(f: *mut FILE) -> c_int;

        /// 由 rusl-stdio 提供 #[no_mangle] 实现
        /// 定义: rusl-stdio/src/__toread.rs
        pub fn __toread(f: *mut FILE) -> c_int;
    }
}

/// rusl 实现的 floatscan 数学辅助函数
/// 注：无 80-bit 中间精度，使用 f64 近似。对子正常数舍入可能略有差异。
#[cfg(feature = "rusl")]
mod floatscan_helpers {
    /// y * 2^e2。
    /// 对应 musl: `scalbnl(t, e2)` with 80-bit intermediate
    pub fn __floatscan_scale(y: f64, e2: i32) -> f64 {
        if y == 0.0 || !y.is_finite() || e2 == 0 {
            return y;
        }
        let bits = y.to_bits();
        let exp = ((bits >> 52) & 0x7FF) as i64;
        // 子正常数：先放大到正常范围
        if exp == 0 {
            // 2^54 = 18014398509481984.0 是 f64 精确可表示值
            let scaled = y * 18014398509481984.0f64;
            return __floatscan_scale(scaled, e2 - 54);
        }
        let new_exp = exp + e2 as i64;
        if new_exp >= 2047 {
            if y.is_sign_positive() { f64::INFINITY } else { f64::NEG_INFINITY }
        } else if new_exp <= 0 {
            if y.is_sign_positive() { 0.0 } else { -0.0 }
        } else {
            f64::from_bits((bits & 0x800F_FFFF_FFFF_FFFF) | ((new_exp as u64) << 52))
        }
    }

    /// a * b。
    /// 对应 musl: `(long double)a * (long double)b` with 80-bit intermediate
    pub fn __floatscan_mul(a: f64, b: f64) -> f64 { a * b }

    /// |x|。
    pub fn __floatscan_abs(x: f64) -> f64 { x.abs() }

    /// 以 y 的符号复制到 x 的绝对值上。
    pub fn __floatscan_copysign(x: f64, y: f64) -> f64 { x.copysign(y) }

    /// 浮点取余。
    pub fn __floatscan_fmod(x: f64, y: f64) -> f64 { x % y }
}

// ---------------------------------------------------------------------------
// 统一导出
// ---------------------------------------------------------------------------

#[cfg(not(feature = "rusl"))]
pub use crate::import::errno::__errno_location;
#[cfg(not(feature = "rusl"))]
pub use crate::import::stdio::__uflow;
#[cfg(not(feature = "rusl"))]
pub use crate::import::floatscan_helpers::{
    __floatscan_scale, __floatscan_mul, __floatscan_abs,
    __floatscan_copysign, __floatscan_fmod,
};

#[cfg(feature = "rusl")]
pub use rusl_errno::__errno_location;
#[cfg(feature = "rusl")]
pub use crate::import::stdio::__uflow;
#[cfg(feature = "rusl")]
pub use crate::import::floatscan_helpers::{
    __floatscan_scale, __floatscan_mul, __floatscan_abs,
    __floatscan_copysign, __floatscan_fmod,
};
