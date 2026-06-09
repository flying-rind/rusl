// Syscall return value conversion — exported C ABI symbol.
//
// Corresponds to musl's src/internal/syscall_ret.c
//
// Linux kernel returns errors as values in [-4095, -1] (unsigned: [0xfffff001, 0xffffffff]).
// This function checks for error, sets errno, and returns -1.
// On success, the result is returned as-is.

use core::ffi::{c_int, c_long, c_ulong};
use rusl_errno::__errno_location;

/// Convert raw kernel syscall return value to libc convention.
///
/// If `r` > `-4096UL` (i.e., r is in the error range), sets errno = -r and returns -1.
/// Otherwise returns r as a signed long.
#[no_mangle]
pub extern "C" fn __syscall_ret(r: c_ulong) -> c_long {
    if r > (0usize.wrapping_sub(4096)) as c_ulong {
        let errno_val = -(r as c_long);
        let errno_ptr = __errno_location();
        unsafe {*errno_ptr = errno_val as c_int;}
        -1
    } else {
        r as c_long
    }
}
