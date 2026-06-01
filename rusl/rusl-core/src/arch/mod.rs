// Architecture detection and re-export.
// Each arch module provides: __syscall0..6, and atomic primitives.

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

// Add more architectures here as they are implemented:
// #[cfg(target_arch = "arm")]    mod arm;    pub use arm::*;
// #[cfg(target_arch = "riscv64")] mod riscv64; pub use riscv64::*;
// ...