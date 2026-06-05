//! 对应musl/src/errno

mod __errno_location;
mod strerror;

pub use __errno_location::*;
pub use strerror::*;