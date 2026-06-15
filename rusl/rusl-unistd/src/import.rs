//! 声明所有依赖其他模块的接口
//!
//! 当不开启 rusl feature 时，使用 musl 的 C 接口

// ========== 系统调用 ==========
#[cfg(feature = "rusl")]
pub use rusl_internal::do_syscall;

#[cfg(not(feature = "rusl"))]
mod internal {
    pub use rusl_syscall::do_syscall;
}
#[cfg(not(feature = "rusl"))]
pub use internal::do_syscall;
