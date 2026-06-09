/// 模块: strerror_test
/// `strerror` / `strerror_l` 集成测试
///
/// 基于 `spec/errno/strerror.md` 规约生成。
///
/// ## 测试覆盖
///
/// - strerror 基本功能: 已知错误码返回正确消息
/// - 错误码 0 返回 "No error information"
/// - 未知/越界错误码返回回退消息
/// - 负错误码返回回退消息
/// - 返回值始终非空
/// - strerror 不修改 errno
/// - strerror_l 与 strerror 行为一致 (rusl Stage 0 忽略 locale)
/// - 返回字符串 NUL 结尾验证

use core::ffi::{c_char, c_int, CStr};

use rusl::api::errno::{__errno_location, strerror, strerror_l};
use test_framework::test;

// ===========================================================================
// 辅助函数
// ===========================================================================

/// 比较由 strerror 返回的 C 字符串与预期的 Rust 字节切片。
///
/// 返回 `true` 表字符串完全匹配 (含 NUL 结尾)。
unsafe fn strerror_eq(e: c_int, expected: &[u8]) -> bool {
    let ptr = strerror(e);
    if ptr.is_null() {
        return false;
    }
    let mut i: usize = 0;
    while i < expected.len() {
        if unsafe { *(ptr as *const u8).add(i) } != expected[i] {
            return false;
        }
        i += 1;
    }
    // 检查 NUL 结尾
    unsafe { *(ptr as *const u8).add(i) == 0 }
}

/// 比较两个 strerror 调用的结果是否逐字节相等 (含 NUL)。
unsafe fn strerror_cmp(a: *mut c_char, b: *mut c_char) -> bool {
    let mut i: usize = 0;
    loop {
        let ba = unsafe { *(a as *const u8).add(i) };
        let bb = unsafe { *(b as *const u8).add(i) };
        if ba != bb {
            return false;
        }
        if ba == 0 {
            return true;
        }
        i += 1;
    }
}

// ===========================================================================
// strerror 基本功能测试 — 错误码 0
// ===========================================================================

test!("test_strerror_zero" {
    let msg = strerror(0);
    assert!(!msg.is_null(), "strerror(0) returned null");
    assert!(unsafe { strerror_eq(0, b"No error information") },
        "strerror(0) did not return 'No error information'");
});

test!("test_strerror_zero_via_cstr" {
    let msg = strerror(0);
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"No error information",
        "strerror(0) mismatch via CStr");
});

// ===========================================================================
// strerror 基本功能测试 — 各标准错误码
// ===========================================================================

test!("test_strerror_eperm" {
    let msg = strerror(1);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(1, b"Operation not permitted") });
});

test!("test_strerror_enoent" {
    let msg = strerror(2);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(2, b"No such file or directory") });
});

test!("test_strerror_esrch" {
    let msg = strerror(3);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(3, b"No such process") });
});

test!("test_strerror_eintr" {
    let msg = strerror(4);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(4, b"Interrupted system call") });
});

test!("test_strerror_eio" {
    let msg = strerror(5);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(5, b"I/O error") });
});

test!("test_strerror_enxio" {
    let msg = strerror(6);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(6, b"No such device or address") });
});

test!("test_strerror_ebadf" {
    let msg = strerror(9);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(9, b"Bad file descriptor") });
});

test!("test_strerror_echild" {
    let msg = strerror(10);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(10, b"No child process") });
});

test!("test_strerror_eagain" {
    let msg = strerror(11);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(11, b"Resource temporarily unavailable") });
});

test!("test_strerror_enomem" {
    let msg = strerror(12);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(12, b"Out of memory") });
});

test!("test_strerror_eacces" {
    let msg = strerror(13);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(13, b"Permission denied") });
});

test!("test_strerror_efault" {
    let msg = strerror(14);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(14, b"Bad address") });
});

test!("test_strerror_einval" {
    let msg = strerror(22);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(22, b"Invalid argument") });
});

test!("test_strerror_efbig" {
    let msg = strerror(27);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(27, b"File too large") });
});

test!("test_strerror_emlink" {
    let msg = strerror(31);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(31, b"Too many links") });
});

test!("test_strerror_epipe" {
    let msg = strerror(32);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(32, b"Broken pipe") });
});

test!("test_strerror_edom" {
    let msg = strerror(33);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(33, b"Domain error") });
});

test!("test_strerror_erange" {
    let msg = strerror(34);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(34, b"Result not representable") });
});

test!("test_strerror_eilseq" {
    let msg = strerror(84);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(84, b"Illegal byte sequence") });
});

test!("test_strerror_eopnotsupp" {
    let msg = strerror(95);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(95, b"Not supported") });
});

test!("test_strerror_etimedout" {
    let msg = strerror(110);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(110, b"Operation timed out") });
});

test!("test_strerror_econnrefused" {
    let msg = strerror(111);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(111, b"Connection refused") });
});

test!("test_strerror_ecanceled" {
    let msg = strerror(125);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(125, b"Operation canceled") });
});

test!("test_strerror_enotrecoverable" {
    let msg = strerror(131);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(131, b"State not recoverable") });
});

// ===========================================================================
// strerror 边界和越界测试
// ===========================================================================

test!("test_strerror_negative" {
    let msg = strerror(-1);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(-1, b"No error information") });
});

test!("test_strerror_negative_large" {
    let msg = strerror(-100);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(-100, b"No error information") });
});

test!("test_strerror_negative_min" {
    let msg = strerror(i32::MIN);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(i32::MIN, b"No error information") });
});

test!("test_strerror_just_beyond_table" {
    let msg = strerror(132);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(132, b"No error information") });
});

test!("test_strerror_way_out_of_range" {
    let msg = strerror(9999);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(9999, b"No error information") });
});

test!("test_strerror_int_max" {
    let msg = strerror(i32::MAX);
    assert!(!msg.is_null());
    assert!(unsafe { strerror_eq(i32::MAX, b"No error information") });
});

// ===========================================================================
// strerror 不变性测试
// ===========================================================================

test!("test_strerror_never_returns_null" {
    let test_cases: &[c_int] = &[
        i32::MIN, -9999, -100, -1,
        0, 1, 2, 3, 13, 22, 32, 84, 110, 111, 125, 131,
        132, 200, 500, 9999, i32::MAX,
    ];
    for &ec in test_cases {
        let msg = strerror(ec);
        assert!(!msg.is_null(), "strerror({}) returned null", ec);
    }
});

test!("test_strerror_does_not_modify_errno" {
    let errno_ptr = __errno_location();
    unsafe { core::ptr::write(errno_ptr, 42) };
    let _ = strerror(0);
    let _ = strerror(2);
    let _ = strerror(9999);
    let val = unsafe { core::ptr::read(errno_ptr) };
    assert_eq!(val, 42, "strerror modified errno, expected 42, got {}", val);
});

test!("test_strerror_consistent_return" {
    for &ec in &[0, 2, 13, 22, 32, 110, 111, 131] {
        let msg1 = strerror(ec);
        let msg2 = strerror(ec);
        assert!(!msg1.is_null());
        assert!(!msg2.is_null());
        assert!(unsafe { strerror_cmp(msg1, msg2) },
            "strerror({}) returned inconsistent results", ec);
    }
});

// ===========================================================================
// strerror_l 基本测试
// ===========================================================================
// rusl Stage 0: strerror_l 忽略 locale 参数, 传入 null 即可。

test!("test_strerror_l_zero" {
    let msg = strerror_l(0, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"No error information",
        "strerror_l(0) mismatch");
});

test!("test_strerror_l_enoent" {
    let msg = strerror_l(2, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"No such file or directory");
});

test!("test_strerror_l_eacces" {
    let msg = strerror_l(13, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"Permission denied");
});

test!("test_strerror_l_einval" {
    let msg = strerror_l(22, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"Invalid argument");
});

test!("test_strerror_l_epipe" {
    let msg = strerror_l(32, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"Broken pipe");
});

test!("test_strerror_l_negative" {
    let msg = strerror_l(-1, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"No error information");
});

test!("test_strerror_l_out_of_range" {
    let msg = strerror_l(9999, core::ptr::null_mut());
    assert!(!msg.is_null());
    let cstr = unsafe { CStr::from_ptr(msg) };
    assert_eq!(cstr.to_bytes(), b"No error information");
});

test!("test_strerror_l_never_returns_null" {
    let loc = core::ptr::null_mut();
    let test_cases: &[c_int] = &[-1, 0, 1, 2, 13, 22, 32, 84, 110, 111, 131, 132, 9999];
    for &ec in test_cases {
        let msg = strerror_l(ec, loc);
        assert!(!msg.is_null(), "strerror_l({}) returned null", ec);
    }
});

// ===========================================================================
// strerror_l 与 strerror 一致性 (逐字节比较)
// ===========================================================================

test!("test_strerror_l_matches_strerror_all_codes" {
    // 对已知错误码, strerror_l (Stage 0) 应返回与 strerror 完全相同的字符串
    let loc = core::ptr::null_mut();
    let test_cases: &[c_int] = &[
        -1, 0, 1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14,
        22, 27, 31, 32, 33, 34,
        84, 95, 110, 111, 125, 131,
        132, 9999,
    ];
    for &ec in test_cases {
        let msg1 = strerror(ec);
        let msg2 = strerror_l(ec, loc);
        assert!(!msg1.is_null(), "strerror({}) returned null", ec);
        assert!(!msg2.is_null(), "strerror_l({}) returned null", ec);
        assert!(unsafe { strerror_cmp(msg1, msg2) },
            "strerror_l({}) differs from strerror({})", ec, ec);
    }
});

// ===========================================================================
// strerror_l 不变性测试
// ===========================================================================

test!("test_strerror_l_does_not_modify_errno" {
    let errno_ptr = __errno_location();
    unsafe { core::ptr::write(errno_ptr, 99) };
    let _ = strerror_l(2, core::ptr::null_mut());
    let _ = strerror_l(13, core::ptr::null_mut());
    let val = unsafe { core::ptr::read(errno_ptr) };
    assert_eq!(val, 99, "strerror_l modified errno, expected 99, got {}", val);
});

// ===========================================================================
// 返回字符串格式验证
// ===========================================================================

test!("test_strerror_nul_terminated" {
    let test_cases: &[c_int] = &[0, 1, 2, 13, 22, 32, 84, 110, 111, 125, 131];
    for &ec in test_cases {
        let msg = strerror(ec);
        assert!(!msg.is_null());
        let cstr = unsafe { CStr::from_ptr(msg) };
        let bytes = cstr.to_bytes();
        assert!(!bytes.is_empty(), "strerror({}) returned empty string", ec);
        let last = bytes[bytes.len() - 1];
        assert!(last >= 32 && last <= 126,
            "strerror({}) last byte not printable: {}", ec, last);
    }
});

test!("test_strerror_no_leading_trailing_whitespace" {
    for &ec in &[0, 1, 2, 13, 22, 32, 84, 111] {
        let msg = strerror(ec);
        let cstr = unsafe { CStr::from_ptr(msg) };
        let bytes = cstr.to_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        assert_ne!(first, b' ', "strerror({}) starts with space", ec);
        assert_ne!(last, b' ', "strerror({}) ends with space", ec);
    }
});