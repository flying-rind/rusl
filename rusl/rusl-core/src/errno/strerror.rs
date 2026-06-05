//! strerror / strerror\_l — Rust 实现 musl libc 错误消息映射。
//!
//! 将 errno 错误码映射为人可读的错误描述字符串。
//!
//! # 内部数据
//!
//! 错误消息数据从 musl `__strerror.h` 中提取，编译为静态只读数据:
//! - [`ERRMSG_DATA`] — 所有消息按 NUL 结尾拼接的字节数组
//! - [`ERRMSG_IDX`]  — errno 值到字节偏移量的索引表
//!
//! 未知错误码回退到 `"No error information"` (OFFSET 0)。
//!
//! # Stage 0
//!
//! `strerror_l` 的 locale 参数被忽略，直接查表返回原始英文消息。
//! 未来 Stage 将集成 `LCTRANS` 进行 locale 消息翻译。

use core::ffi::{c_char, c_int};
use crate::locale_t;

// ===========================================================================
// 错误消息数据 (从 musl __strerror.h 提取)
// ===========================================================================

/// 按定义顺序拼接的错误消息字节数组 (NUL 结尾)。
///
/// 第一条消息 (偏移 0) 固定为 `"No error information\0"`，
/// 作为所有未定义错误码的统一回退消息。
///
/// 数据长度: 1960 字节，共 95 条消息。
pub(crate) static ERRMSG_DATA: &[u8] = b"\
No error information\0Illegal byte sequence\0Domain error\0Result not \
representable\0Not a tty\0Permission denied\0Operation not permitted\0\
No such file or directory\0No such process\0File exists\0Value too la\
rge for data type\0No space left on device\0Out of memory\0Resource b\
usy\0Interrupted system call\0Resource temporarily unavailable\0Inval\
id seek\0Cross-device link\0Read-only file system\0Directory not empt\
y\0Connection reset by peer\0Operation timed out\0Connection refused\0\
Host is down\0Host is unreachable\0Address in use\0Broken pipe\0I/O e\
rror\0No such device or address\0Block device required\0No such devic\
e\0Not a directory\0Is a directory\0Text file busy\0Exec format error\
\0Invalid argument\0Argument list too long\0Symbolic link loop\0Filen\
ame too long\0Too many open files in system\0No file descriptors avai\
lable\0Bad file descriptor\0No child process\0Bad address\0File too l\
arge\0Too many links\0No locks available\0Resource deadlock would occ\
ur\0State not recoverable\0Previous owner died\0Operation canceled\0F\
unction not implemented\0No message of desired type\0Identifier remov\
ed\0Device not a stream\0No data available\0Device timeout\0Out of st\
reams resources\0Link has been severed\0Protocol error\0Bad message\0\
File descriptor in bad state\0Not a socket\0Destination address requi\
red\0Message too large\0Protocol wrong type for socket\0Protocol not \
available\0Protocol not supported\0Socket type not supported\0Not sup\
ported\0Protocol family not supported\0Address family not supported b\
y protocol\0Address not available\0Network is down\0Network unreachab\
le\0Connection reset by network\0Connection aborted\0No buffer space \
available\0Socket is connected\0Socket not connected\0Cannot send aft\
er socket shutdown\0Operation already in progress\0Operation in progr\
ess\0Stale file handle\0Data consistency error\0Resource not availabl\
e\0Remote I/O error\0Quota exceeded\0No medium found\0Wrong medium ty\
pe\0Multihop attempted\0Required key not available\0Key has expired\0\
Key has been revoked\0Key was rejected by service\0";

/// 错误码到 ERRMSG_DATA 偏移量的索引表。
///
/// `ERRMSG_IDX[e]` 给出错误码 `e` 在 [`ERRMSG_DATA`] 中的起始字节偏移量。
/// 数组长度 132 (覆盖最大 errno 值: ENOTRECOVERABLE=131)。
/// 未定义的错误码映射到偏移量 0 (回退消息 `"No error information"`)。
pub(crate) static ERRMSG_IDX: [u16; 132] = [
        0,   109,   133,   159,   269,   523,   533,   677,
      642,   797,   817,   293,   241,    91,   834,   559,
      255,   175,   339,   581,   596,   612,   660,   737,
      767,    81,   627,   846,   217,   326,   357,   861,
      511,    43,    56,   895,   719,   876,   986,   379,
      700,     0,  1011,  1038,     0,     0,     0,     0,
        0,     0,     0,     0,     0,     0,     0,     0,
        0,     0,     0,     0,  1057,  1077,  1095,  1110,
        0,     0,     0,  1135,     0,     0,     0,  1157,
     1849,     0,  1172,   187,     0,  1184,     0,     0,
        0,     0,     0,     0,    21,     0,     0,     0,
     1213,  1226,  1255,  1273,  1304,  1327,  1350,  1376,
     1390,  1420,   496,  1461,  1483,  1499,  1519,  1547,
      399,  1566,  1592,  1612,  1633,     0,   424,   444,
      463,   476,  1667,  1697,  1719,  1737,     0,  1760,
        0,  1783,  1800,  1815,  1831,   967,  1868,  1895,
     1911,  1932,   947,   925,
];

// ===========================================================================
// 内部辅助函数
// ===========================================================================

/// 根据 errno 错误码返回对应的错误描述字符串指针。
fn errno_to_message(e: c_int) -> *const c_char {
    // MIPS EDQUOT 兼容修正 — 仅在 MIPS 架构上编译
    #[cfg(any(target_arch = "mips", target_arch = "mips64"))]
    let idx: c_int = {
        if e == 109 {
            0
        } else if e == 1133 {
            109
        } else {
            e
        }
    };

    #[cfg(not(any(target_arch = "mips", target_arch = "mips64")))]
    let idx: c_int = e;

    // 边界检查: 只有合法范围才查表
    if idx >= 0 && (idx as usize) < ERRMSG_IDX.len() {
        let offset = ERRMSG_IDX[idx as usize] as usize;
        // SAFETY: ERRMSG_IDX 中的偏移量均在 ERRMSG_DATA 范围内
        ERRMSG_DATA.as_ptr().wrapping_add(offset) as *const c_char
    } else {
        // 超出范围: 回退到索引 0 "No error information"
        ERRMSG_DATA.as_ptr() as *const c_char
    }
}

// ===========================================================================
// strerror / strerror_l — C ABI 导出的错误消息函数
// ===========================================================================

/// 返回 errno 错误码 `e` 对应的可读错误描述字符串。
///
/// 返回的指针指向静态只读数据，调用者不应修改或释放。
///
/// # C 签名
///
/// ```c
/// char *strerror(int errnum);
/// ```
#[no_mangle]
pub extern "C" fn strerror(e: c_int) -> *mut c_char {
    errno_to_message(e) as *mut c_char
}

/// 返回 errno 错误码 `e` 在指定 locale 下的可读错误描述字符串。
///
/// Stage 0: `loc` 参数被忽略, 直接查表返回原始英文消息。
///
/// 返回的指针指向静态只读数据，调用者不应修改或释放。
///
/// # C 签名
///
/// ```c
/// char *strerror_l(int errnum, locale_t loc);
/// ```
#[no_mangle]
pub extern "C" fn strerror_l(e: c_int, _loc: locale_t) -> *mut c_char {
    errno_to_message(e) as *mut c_char
}

// ===========================================================================
// 单元测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test;

    test!("test_errmsg_data_accessible" {
        let ptr = ERRMSG_DATA.as_ptr();
        assert!(!ptr.is_null(), "ERRMSG_DATA pointer is null");
        assert!(ERRMSG_DATA.len() > 0, "ERRMSG_DATA is empty");
    });

    /// 辅助函数: 比较 C 字符串与预期字节序列
    unsafe fn cstr_eq(ptr: *const u8, expected: &[u8]) -> bool {
        let mut i: usize = 0;
        while i < expected.len() {
            if unsafe { *ptr.add(i) } != expected[i] {
                return false;
            }
            i += 1;
        }
        // 检查 NUL 结尾
        unsafe { *ptr.add(i) == 0 }
    }

    test!("test_strerror_zero" {
        let msg = strerror(0);
        assert!(!msg.is_null());
        assert!(unsafe { cstr_eq(msg as *const u8, b"No error information") });
    });

    test!("test_strerror_epipe" {
        let msg = strerror(32);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Broken pipe") });
    });

    test!("test_strerror_enoent" {
        let msg = strerror(2);
        assert!(!msg.is_null(), "null ptr");
        // 只需检查首个字节
        let first = unsafe { *(msg as *const u8) };
        assert_eq!(first, b'N', "first byte mismatch");
    });

    test!("test_strerror_einval" {
        let msg = strerror(22);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Invalid argument") });
    });

    test!("test_strerror_eacces" {
        // 直接读 ERRMSG_DATA
        let ptr = ERRMSG_DATA.as_ptr().wrapping_add(ERRMSG_IDX[13] as usize);
        let first = unsafe { *ptr };
        assert_eq!(first, b'P', "direct read first byte mismatch");
        let msg = strerror(13);
        assert!(!msg.is_null(), "strerror(13) returned null");
        let first2 = unsafe { *(msg as *const u8) };
        assert_eq!(first2, b'P', "strerror first byte mismatch");
    });

    test!("test_strerror_negative" {
        let msg = strerror(-1);
        assert!(unsafe { cstr_eq(msg as *const u8, b"No error information") });
    });

    test!("test_strerror_out_of_range" {
        let msg = strerror(9999);
        assert!(unsafe { cstr_eq(msg as *const u8, b"No error information") });
    });

    test!("test_strerror_edge_of_table" {
        let msg = strerror(131);
        assert!(unsafe { cstr_eq(msg as *const u8, b"State not recoverable") });
    });

    test!("test_strerror_just_beyond_table" {
        let msg = strerror(132);
        assert!(unsafe { cstr_eq(msg as *const u8, b"No error information") });
    });

    test!("test_strerror_eilseq" {
        let msg = strerror(84);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Illegal byte sequence") });
    });

    test!("test_strerror_edom" {
        let msg = strerror(33);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Domain error") });
    });

    test!("test_strerror_erange" {
        let msg = strerror(34);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Result not representable") });
    });

    test!("test_strerror_eagain" {
        let msg = strerror(11);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Resource temporarily unavailable") });
    });

    test!("test_strerror_enomem" {
        let msg = strerror(12);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Out of memory") });
    });

    test!("test_strerror_ebadf" {
        let msg = strerror(9);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Bad file descriptor") });
    });

    test!("test_strerror_echild" {
        let msg = strerror(10);
        assert!(unsafe { cstr_eq(msg as *const u8, b"No child process") });
    });

    test!("test_strerror_efault" {
        let msg = strerror(14);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Bad address") });
    });

    test!("test_strerror_efbig" {
        let msg = strerror(27);
        assert!(unsafe { cstr_eq(msg as *const u8, b"File too large") });
    });

    test!("test_strerror_emlink" {
        let msg = strerror(31);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Too many links") });
    });

    test!("test_strerror_econnrefused" {
        let msg = strerror(111);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Connection refused") });
    });

    test!("test_strerror_etimedout" {
        let msg = strerror(110);
        assert!(unsafe { cstr_eq(msg as *const u8, b"Operation timed out") });
    });

    test!("test_strerror_unknown_errno_should_fallback" {
        let msg = strerror(41);
        assert!(unsafe { cstr_eq(msg as *const u8, b"No error information") });
    });

    test!("test_strerror_l_same_as_strerror" {
        for errno_val in &[0, 2, 13, 22, 32, 84, 110, 111, 131] {
            let msg1 = strerror(*errno_val);
            let msg2 = strerror_l(*errno_val, core::ptr::null_mut());
            let mut i: usize = 0;
            loop {
                let b1 = unsafe { *(msg1 as *const u8).add(i) };
                let b2 = unsafe { *(msg2 as *const u8).add(i) };
                assert_eq!(b1, b2, "strerror and strerror_l mismatch for errno {} at byte {}", errno_val, i);
                if b1 == 0 { break; }
                i += 1;
            }
        }
    });

    test!("test_strerror_l_with_locale_ignored" {
        let msg1 = strerror_l(2, core::ptr::null_mut());
        let msg2 = strerror_l(2, 0xDEAD_BEEF as *mut core::ffi::c_void);
        let mut i: usize = 0;
        loop {
            let b1 = unsafe { *(msg1 as *const u8).add(i) };
            let b2 = unsafe { *(msg2 as *const u8).add(i) };
            assert_eq!(b1, b2, "locale should be ignored in Stage 0, byte {}", i);
            if b1 == 0 { break; }
            i += 1;
        }
    });

    test!("test_strerror_ptr_not_null" {
        for errno_val in &[-1, 0, 1, 2, 13, 22, 32, 84, 131, 9999] {
            let msg = strerror(*errno_val);
            assert!(!msg.is_null(), "strerror({}) returned null", errno_val);
        }
    });

    test!("test_strerror_l_ptr_not_null" {
        for errno_val in &[-1, 0, 1, 2, 13, 22, 32, 84, 131, 9999] {
            let msg = strerror_l(*errno_val, core::ptr::null_mut());
            assert!(!msg.is_null(), "strerror_l({}) returned null", errno_val);
        }
    });

    test!("test_errmsg_idx_consistency" {
        // 抽查代表性条目验证 ERRMSG_IDX 偏移量正确
        let sample: &[(c_int, &[u8])] = &[
            (0, b"No error information"),
            (1, b"Operation not permitted"),
            (2, b"No such file or directory"),
            (13, b"Permission denied"),
            (22, b"Invalid argument"),
            (32, b"Broken pipe"),
            (84, b"Illegal byte sequence"),
            (95, b"Not supported"),
            (110, b"Operation timed out"),
            (111, b"Connection refused"),
            (125, b"Operation canceled"),
            (131, b"State not recoverable"),
        ];
        for &(errno_val, expected_msg) in sample {
            let offset = ERRMSG_IDX[errno_val as usize] as usize;
            let ptr = ERRMSG_DATA.as_ptr().wrapping_add(offset);
            assert!(unsafe { cstr_eq(ptr, expected_msg) },
                "ERRMSG_IDX entry for errno {} at offset {} has wrong message",
                errno_val, offset);
        }
    });

    test!("test_errmsg_data_nul_terminated" {
        let last_byte = ERRMSG_DATA[ERRMSG_DATA.len() - 1];
        assert_eq!(last_byte, 0, "ERRMSG_DATA must end with NUL");
    });
}