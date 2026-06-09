//! strsignal — 返回信号编号 signum 对应的描述字符串。

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;

/// 信号到消息的简单映射（均为 null 终止的 C 字符串）
const SIGNAL_MSGS: &[(core::ffi::c_int, &[u8])] = &[
    (1, b"Hangup\0"),
    (2, b"Interrupt\0"),
    (3, b"Quit\0"),
    (4, b"Illegal instruction\0"),
    (5, b"Trace/breakpoint trap\0"),
    (6, b"Aborted\0"),
    (7, b"Bus error\0"),
    (8, b"Floating point exception\0"),
    (9, b"Killed\0"),
    (10, b"User defined signal 1\0"),
    (11, b"Segmentation fault\0"),
    (12, b"User defined signal 2\0"),
    (13, b"Broken pipe\0"),
    (14, b"Alarm clock\0"),
    (15, b"Terminated\0"),
];

/// 全局静态缓冲区用于存储未知信号号的消息
static mut UNKNOWN_BUF: [u8; 32] = [0u8; 32];

/// strsignal — 返回信号编号 signum 对应的描述字符串。
///
/// # Safety
/// - 无（signum 可为任意整数）
#[no_mangle]
#[allow(static_mut_refs)]
pub extern "C" fn strsignal(signum: core::ffi::c_int) -> *mut core::ffi::c_char {
    if let Some(&(_, msg)) = SIGNAL_MSGS.iter().find(|&&(sig, _)| sig == signum) {
        return msg.as_ptr() as *mut core::ffi::c_char;
    }
    // SAFETY: 唯一访问全局 static mut UNKNOWN_BUF 的地方，无并发冲突
    unsafe {
        let buf = UNKNOWN_BUF.as_mut_ptr();
        let msg = b"Unknown signal ";
        let mut i = 0;
        for &b in msg {
            *buf.add(i) = b;
            i += 1;
        }
        let mut n = signum;
        if n < 0 {
            *buf.add(i) = b'-';
            i += 1;
            n = -n;
        }
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
            *buf.add(i) = digits[nd];
            i += 1;
        }
        *buf.add(i) = 0;
        buf as *mut core::ffi::c_char
    }
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
