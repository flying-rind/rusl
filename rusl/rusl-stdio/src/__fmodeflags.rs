//! __fmodeflags — 将 fopen mode 字符串转换为 open() 系统调用标志位。
//! 对应 musl src/stdio/__fmodeflags.c

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

// Linux open flags (x86_64 / aarch64)
const O_RDONLY: c_int = 0;
const O_WRONLY: c_int = 1;
const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;   // 0100
const O_EXCL: c_int = 0o200;    // 0200
const O_TRUNC: c_int = 0o1000;  // 01000
const O_APPEND: c_int = 0o2000; // 02000
const O_CLOEXEC: c_int = 0o2000000; // 02000000

/// 内部函数：将 mode 字符串（如 "r", "w+", "a+xe"）转换为 open() 标志位。
/// [Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fmodeflags(mode: *const c_char) -> c_int {
    unsafe {
        let mut flags: c_int;

        // 检查是否包含 '+'
        let mut has_plus = false;
        let mut has_x = false;
        let mut has_e = false;
        let mut i = 0;
        let first = *mode;
        if first == 0 {
            return -1;
        }

        loop {
            let ch = *mode.add(i);
            if ch == 0 { break; }
            if ch == b'+' as c_char { has_plus = true; }
            if ch == b'x' as c_char { has_x = true; }
            if ch == b'e' as c_char { has_e = true; }
            i += 1;
        }

        if has_plus {
            flags = O_RDWR;
        } else if first == b'r' as c_char {
            flags = O_RDONLY;
        } else {
            flags = O_WRONLY;
        }

        if has_x { flags |= O_EXCL; }
        if has_e { flags |= O_CLOEXEC; }
        if first != b'r' as c_char { flags |= O_CREAT; }
        if first == b'w' as c_char { flags |= O_TRUNC; }
        if first == b'a' as c_char { flags |= O_APPEND; }

        flags
    }
}
