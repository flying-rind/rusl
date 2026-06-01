//! errno/strerror — Rusl implementation of libc errno functions.
//! Auto-generated from spec files.

#![allow(dead_code, unused_imports)]

mod __errno_location;
mod strerror;

pub use __errno_location::*;
pub use strerror::*;