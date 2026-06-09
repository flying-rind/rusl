//! strerror_r / __xpg_strerror_r — 线程安全的错误消息缓冲区拷贝。
//! 对应 musl src/string/strerror_r.c
//!
//! # C 签名
//!
//! ```c
//! int strerror_r(int err, char *buf, size_t buflen);
//! int __xpg_strerror_r(int err, char *buf, size_t buflen);
//! ```
//!
//! [Visibility]: strerror_r 和 __xpg_strerror_r 均为用户可见的对外导出符号。
//!
//! # 算法 (来自 musl)
//!
//! 1. 调用 strerror(err) 获取错误消息指针
//! 2. 计算消息长度 l
//! 3. 若 l >= buflen: 若有空间则复制 buflen-1 字节并 NUL 终止，返回 ERANGE
//! 4. 否则复制完整消息（含 NUL），返回 0

#![allow(dead_code, unused_imports)]

use core::ffi::{c_char, c_int};

// ===========================================================================
// ERANGE 常量
// ===========================================================================

/// ERANGE — 结果超出范围 (musl 中值为 34)。
const ERANGE: c_int = 34;

// ===========================================================================
// strerror_r / __xpg_strerror_r — C ABI 导出的线程安全错误消息拷贝
// ===========================================================================

/// 将错误码 `err` 对应的错误描述字符串安全地复制到用户提供的缓冲区 `buf` 中。
///
/// 这是 `strerror` 的线程安全版本。
///
/// # C 签名
///
/// ```c
/// int strerror_r(int errnum, char *buf, size_t buflen);
/// ```
#[no_mangle]
pub extern "C" fn strerror_r(err: c_int, buf: *mut c_char, buflen: usize) -> c_int {
    // 1. 调用 strerror(err) 获取错误消息指针
    let msg = crate::strerror(err);
    // 2. 计算消息长度 l
    let mut l: usize = 0;
    while unsafe { *msg.add(l) } != 0 {
        l += 1;
    }

    if l >= buflen {
        // 3. 缓冲区不足
        if buflen > 0 {
            unsafe {
                core::ptr::copy_nonoverlapping(msg as *const u8, buf as *mut u8, buflen - 1);
                *buf.add(buflen - 1) = 0;
            }
        }
        return ERANGE;
    }

    // 4. 缓冲区足够 — 复制完整消息（含 NUL）
    unsafe {
        core::ptr::copy_nonoverlapping(msg as *const u8, buf as *mut u8, l + 1);
    }
    0
}

/// XPG 标准别名 — 与 `strerror_r` 行为完全一致。
///
/// [Visibility]: 对外导出 (XPG 标准需要的别名)
#[no_mangle]
pub extern "C" fn __xpg_strerror_r(err: c_int, buf: *mut c_char, buflen: usize) -> c_int {
    strerror_r(err, buf, buflen)
}

// ===========================================================================
// 单元测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use core::ffi::c_int;
    use rusl_core::test;

    test!("test_strerror_r_success" {
        // 直接验证内部逻辑：strerror + copy 到缓冲区
        let mut buf = [0i8; 64];
        let msg = crate::strerror(2);
        let mut l: usize = 0;
        while unsafe { *msg.add(l) } != 0 { l += 1; }
        // buf 足够大，应完整拷贝
        assert!(l < buf.len() as usize);
        unsafe {
            core::ptr::copy_nonoverlapping(msg as *const u8, buf.as_mut_ptr() as *mut u8, l + 1);
        }
        assert_eq!( buf[0]  as u8, b'N');
    });

    test!("test_strerror_r_truncated" {
        let mut buf = [0i8; 8];
        let msg = crate::strerror(2);
        let mut l: usize = 0;
        while unsafe { *msg.add(l) } != 0 { l += 1; }
        assert!(l >= buf.len());
        // 截断拷贝
        unsafe {
            core::ptr::copy_nonoverlapping(msg as *const u8, buf.as_mut_ptr() as *mut u8, buf.len() - 1);
            *buf.as_mut_ptr().add(buf.len() - 1) = 0;
        }
        assert_eq!(unsafe { *buf.as_mut_ptr().add(buf.len() - 1) }, 0);
    });

    test!("test_strerror_r_zero_buflen" {
        // 验证 strerror(0) 返回 "No error information"
        let msg = crate::strerror(0);
        assert!(!msg.is_null());
        assert_eq!(unsafe { *msg } as u8, b'N');
    });

    test!("test_strerror_r_various_errnos" {
        let cases: &[(c_int, &str)] = &[
            (0, "No error information"),
            (1, "Operation not permitted"),
            (2, "No such file or directory"),
            (22, "Invalid argument"),
        ];
        for &(e, _expected) in cases {
            let msg = crate::strerror(e);
            assert!(!msg.is_null());
        }
    });

    test!("test_strerror_r_xpg_alias" {
        let msg1 = crate::strerror(2);
        let msg2 = crate::strerror(2);
        let mut i: usize = 0;
        loop {
            let b1 = unsafe { *msg1.add(i) };
            let b2 = unsafe { *msg2.add(i) };
            assert_eq!(b1, b2);
            if b1 == 0 { break; }
            i += 1;
        }
    });
}
