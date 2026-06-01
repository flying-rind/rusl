// Syscall return value conversion — exported C ABI symbol.
//
// Corresponds to musl's src/internal/syscall_ret.c
//
// Linux kernel returns errors as values in [-4095, -1] (unsigned: [0xfffff001, 0xffffffff]).
// This function checks for error, sets errno, and returns -1.
// On success, the result is returned as-is.

use core::ffi::{c_int, c_long, c_ulong};

/// Convert raw kernel syscall return value to libc convention.
///
/// If `r` > `-4096UL` (i.e., r is in the error range), sets errno = -r and returns -1.
/// Otherwise returns r as a signed long.
#[no_mangle]
pub unsafe extern "C" fn __syscall_ret(r: c_ulong) -> c_long {
    // -4096UL = 0xfffffffffffff000 on 64-bit
    // This check works for both 32-bit and 64-bit because errno values
    // are in the range [1, 4095] so negative values = [0xfffff001, 0xffffffff]
    if r > (0usize.wrapping_sub(4096)) as c_ulong {
        let errno_val = -(r as c_long);
        // Call __errno_location to get a pointer to the thread's errno
        let errno_ptr = crate::errno::__errno_location();
        *errno_ptr = errno_val as c_int;
        -1
    } else {
        r as c_long
    }
}