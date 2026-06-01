//! strsignal — 返回信号编号 signum 对应的描述字符串。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 信号到消息的简单映射
const SIGNAL_MSGS: &[(core::ffi::c_int, &[u8])] = &[
    (1, b"Hangup"),
    (2, b"Interrupt"),
    (3, b"Quit"),
    (4, b"Illegal instruction"),
    (5, b"Trace/breakpoint trap"),
    (6, b"Aborted"),
    (7, b"Bus error"),
    (8, b"Floating point exception"),
    (9, b"Killed"),
    (10, b"User defined signal 1"),
    (11, b"Segmentation fault"),
    (12, b"User defined signal 2"),
    (13, b"Broken pipe"),
    (14, b"Alarm clock"),
    (15, b"Terminated"),
];

/// 全局静态缓冲区用于存储未知信号号的消息
static mut UNKNOWN_BUF: [u8; 32] = [0u8; 32];

/// strsignal — 返回信号编号 signum 对应的描述字符串。
///
/// # Safety
/// - 无（signum 可为任意整数）
#[no_mangle]
#[allow(static_mut_refs)]
pub unsafe extern "C" fn strsignal(signum: core::ffi::c_int) -> *mut core::ffi::c_char {
    if let Some(&(_, msg)) = SIGNAL_MSGS.iter().find(|&&(sig, _)| sig == signum) {
        return msg.as_ptr() as *mut core::ffi::c_char;
    }
    // 未知信号，格式化到静态缓冲区
    let msg = b"Unknown signal ";
    let mut i = 0;
    for &b in msg {
        UNKNOWN_BUF[i] = b;
        i += 1;
    }
    // 追加信号号（简单整数格式化）
    let mut n = signum;
    if n < 0 {
        UNKNOWN_BUF[i] = b'-';
        i += 1;
        n = -n;
    }
    // 反转数字
    let mut digits = [0u8; 12];
    let mut nd = 0;
    if n == 0 {
        digits[nd] = b'0';
        nd += 1;
    } else {
        while n > 0 {
            digits[nd] = b'0' + (n % 10) as u8;
            nd += 1;
            n /= 10;
        }
    }
    while nd > 0 {
        nd -= 1;
        UNKNOWN_BUF[i] = digits[nd];
        i += 1;
    }
    UNKNOWN_BUF[i] = 0;
    UNKNOWN_BUF.as_mut_ptr() as *mut core::ffi::c_char
}

/// 安全的 Rust 内部实现。
pub(crate) fn strsignal_impl(signum: core::ffi::c_int) -> &'static core::ffi::CStr {
    // 对于已知信号，使用静态 CStr
    if let Some(&(_, msg)) = SIGNAL_MSGS.iter().find(|&&(sig, _)| sig == signum) {
        return unsafe { core::ffi::CStr::from_ptr(msg.as_ptr() as *const core::ffi::c_char) };
    }
    // 对于未知信号，返回泛化消息
    unsafe { core::ffi::CStr::from_ptr(b"Unknown signal\0".as_ptr() as *const core::ffi::c_char) }
}
