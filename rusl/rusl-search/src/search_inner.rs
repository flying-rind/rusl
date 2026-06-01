//! Search operations — Rusl implementation of libc search.h functions.
//! Auto-generated from spec files.

#![allow(dead_code, unused_imports)]

mod types;
mod hsearch;
mod insque;
mod lsearch;
mod tsearch;
mod tfind;
mod tdelete;
mod tdestroy;
mod twalk;

// Re-export public C ABI symbols
pub use types::{ENTRY, ACTION, VISIT};
pub use hsearch::{hcreate, hdestroy, hsearch};
pub use insque::{insque, remque};
pub use lsearch::{lfind, lsearch};
#[cfg(not(test))]
pub use tsearch::tsearch;
#[cfg(not(test))]
pub use tfind::tfind;
#[cfg(not(test))]
pub use tdelete::tdelete;
#[cfg(not(test))]
pub use tdestroy::tdestroy;
#[cfg(not(test))]
pub use twalk::twalk;