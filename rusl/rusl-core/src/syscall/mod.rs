//! syscall — 系统调用封装。
//! 对应 musl src/internal/syscall.h + arch/*/syscall_arch.h

#![allow(unused_imports)]

pub mod num;
pub mod raw;
pub mod do_syscall;
pub mod syscall_ret;

pub use num::*;
pub use raw::*;
pub use do_syscall::*;
pub use syscall_ret::*;
