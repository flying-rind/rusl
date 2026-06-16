//! thread 集成测试子模块

#[allow(unused)]
pub use rusl::api::thread::*;
#[allow(unused)]
pub use core::ffi::c_int;
#[allow(unused)]
pub use core::ffi::c_uint;
#[allow(unused)]
pub use core::ffi::c_char;
#[allow(unused)]
pub use core::ffi::c_void;
#[allow(unused)]
pub use core::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};

// 常用 errno 常量 (Linux x86_64)
pub const EINVAL: c_int = 22;
pub const EAGAIN: c_int = 11;
pub const EPERM: c_int = 1;
pub const ETIMEDOUT: c_int = 110;
pub const EBUSY: c_int = 16;
pub const ESRCH: c_int = 3;

mod thread_attr;
mod thread_lifecycle;
mod thread_np;
mod mutex;
mod rwlock;
mod cond;
mod barrier;
mod spinlock;
mod once;
mod cancel;
mod sched;
mod signal;
mod tsd;
mod thread_name;
mod sem;
mod c11_threads;
mod atfork;
