//! 声明所有依赖其他模块的接口
//!
//! do_syscall! 宏路由: rusl feature 时用 rusl_internal, 否则用 rusl_syscall

#[cfg(feature = "rusl")]
pub use rusl_internal::do_syscall;

#[cfg(not(feature = "rusl"))]
pub use rusl_syscall::do_syscall;
