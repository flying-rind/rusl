//! stdio — 标准 I/O 实现。
//! 对应 musl src/stdio/ 目录。

#![allow(dead_code, unused_imports)]

pub(crate) mod stdio_impl;
mod fwrite;
mod __towrite;
mod __toread;
mod __uflow;
mod vsnprintf;
mod vfprintf;
mod snprintf;

pub use vsnprintf::vsnprintf;
pub use vfprintf::vfprintf;
pub use __uflow::__uflow;
pub use __toread::__toread;
// snprintf is provided by C wrapper (snprintf_wrapper.c) compiled in build.rs