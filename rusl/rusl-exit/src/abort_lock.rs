//! __abort_lock — abort 锁。
//! 对应 musl `src/exit/abort_lock.c`。

use core::ffi::c_int;

#[no_mangle]
pub static mut __abort_lock: [c_int; 1] = [0];
