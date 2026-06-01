//! strerror_r — 将错误码 err 对应的错误描述字符串安全地复制到用户提供的缓冲区 buf 中（线程安全）。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::ffi::c_int;

/// 错误码到消息的简单映射
const ERROR_MSGS: &[(c_int, &[u8])] = &[
    (0, b"Success"),
    (1, b"Operation not permitted"),
    (2, b"No such file or directory"),
    (3, b"No such process"),
    (4, b"Interrupted system call"),
    (5, b"I/O error"),
    (6, b"No such device or address"),
    (7, b"Argument list too long"),
    (8, b"Exec format error"),
    (9, b"Bad file descriptor"),
    (11, b"Resource temporarily unavailable"),
    (12, b"Cannot allocate memory"),
    (13, b"Permission denied"),
    (14, b"Bad address"),
    (16, b"Device or resource busy"),
    (17, b"File exists"),
    (22, b"Invalid argument"),
    (98, b"Address already in use"),
    (99, b"Cannot assign requested address"),
];

/// strerror_r — 将错误码 err 对应的错误描述字符串安全地复制到用户提供的缓冲区 buf 中（线程安全）。
///
/// # Safety
/// - `buf` 非空或 `buflen == 0`
/// - 当 `buflen > 0` 时，`buf` 至少可写 buflen 字节
#[no_mangle]
pub unsafe extern "C" fn strerror_r(err: core::ffi::c_int, buf: *mut core::ffi::c_char, buflen: usize) -> core::ffi::c_int {
    let msg = ERROR_MSGS.iter().find(|&&(code, _)| code == err)
        .map(|&(_, msg)| msg)
        .unwrap_or(b"Unknown error");
    if buflen == 0 {
        return 0;
    }
    let dst = buf as *mut u8;
    let copy_len = (msg.len()).min(buflen - 1);
    for i in 0..copy_len {
        unsafe { *dst.add(i) = msg[i]; }
    }
    unsafe { *dst.add(copy_len) = 0; }
    0
}

#[no_mangle]
pub unsafe extern "C" fn __xpg_strerror_r(err: core::ffi::c_int, buf: *mut core::ffi::c_char, buflen: usize) -> core::ffi::c_int {
    unsafe { strerror_r(err, buf, buflen) }
}

/// 安全的 Rust 内部实现。
pub(crate) fn strerror_r_impl(err: core::ffi::c_int, buf: &mut [u8]) -> core::ffi::c_int {
    let msg = ERROR_MSGS.iter().find(|&&(code, _)| code == err)
        .map(|&(_, msg)| msg)
        .unwrap_or(b"Unknown error");
    let copy_len = msg.len().min(buf.len().saturating_sub(1));
    buf[..copy_len].copy_from_slice(&msg[..copy_len]);
    if copy_len < buf.len() {
        buf[copy_len] = 0;
    }
    0
}
