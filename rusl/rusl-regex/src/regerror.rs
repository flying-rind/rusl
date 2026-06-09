//! regerror — POSIX 正则表达式错误消息转换。对外导出 C ABI 兼容的 `regerror` 符号。
//!
//! 将 `regcomp()` 或 `regexec()` 返回的 `REG_*` 错误码转换为人类可读的错误消息字符串，
//! 支持 locale 翻译。

#![allow(unused_imports, unused_variables)]

use core::ffi::{c_char, c_int};

/// C ABI 中 `size_t` 的类型别名。
type size_t = usize;

// ============================================================================
// 内部实现：错误消息表
// ============================================================================

/// 错误消息静态表 — 以 `REG_*` 错误码为索引。
///
/// # 不变量
///
/// - 索引顺序必须与 `<regex.h>` 中 `REG_*` 错误码数值严格一致
/// - 对越界错误码，返回 `UNKNOWN_ERROR`
pub(crate) static ERROR_MESSAGES: &[&str] = &[
    "No error",                                    // 索引 0: REG_OK
    "No match",                                    // 索引 1: REG_NOMATCH
    "Invalid regexp",                              // 索引 2: REG_BADPAT
    "Unknown collating element",                   // 索引 3: REG_ECOLLATE
    "Unknown character class name",                // 索引 4: REG_ECTYPE
    "Trailing backslash",                          // 索引 5: REG_EESCAPE
    "Invalid back reference",                      // 索引 6: REG_ESUBREG
    "Missing ']'",                                 // 索引 7: REG_EBRACK
    "Missing ')'",                                 // 索引 8: REG_EPAREN
    "Missing '}'",                                 // 索引 9: REG_EBRACE
    "Invalid contents of {}",                      // 索引 10: REG_BADBR
    "Invalid character range",                     // 索引 11: REG_ERANGE
    "Out of memory",                               // 索引 12: REG_ESPACE
    "Repetition not preceded by valid expression", // 索引 13: REG_BADRPT
];

/// 错误码越界时的兜底消息。
pub(crate) static UNKNOWN_ERROR: &str = "Unknown error";

// ============================================================================
// regerror (对外导出)
// ============================================================================

/// POSIX `regerror()` — 将错误码转换为人类可读的错误消息字符串。
///
/// [Visibility]: Public — POSIX.1-2001 标准函数，`<regex.h>` 声明。
///
/// # Safety
///
/// 调用者必须确保：
/// - 若 `errbuf` 非 `null()` 且 `errbuf_size > 0`，则 `errbuf` 指向的缓冲区
///   至少有 `errbuf_size` 字节可写。
/// - 若 `errbuf_size == 0`，`errbuf` 可为 `null()`。
///
/// # 后置条件
///
/// **Case 1: `errbuf` 非 `null()` 且 `errbuf_size > 0`**
/// - 错误消息写入缓冲区（以 `\0` 结尾）
/// - 若返回值 `<= errbuf_size`，完整消息被写入
/// - 若返回值 `> errbuf_size`，消息被截断至 `errbuf_size-1` 字节
///
/// **Case 2: `errbuf == null()` 或 `errbuf_size == 0`**
/// - 不发生写入操作
/// - 返回值仍为完整消息所需的总字符数（含 `\0`）
///
/// # 返回值
///
/// 完整写入消息所需的字符数（含结尾 `\0`）。
///
/// # 实现说明
///
/// musl 实现中 `preg` 参数被完全忽略。消息通过直接数组索引定位（而非
/// C 实现的线性扫描），索引与 `REG_*` 错误码数值严格对应。
#[no_mangle]
pub extern "C" fn regerror(
    errcode: c_int,
    _preg: *const super::regcomp::regex_t,
    errbuf: *mut c_char,
    errbuf_size: size_t,
) -> size_t {
    // 1. 定位错误消息（C 实现使用线性扫描，Rust 使用 O(1) 数组索引）
    let msg: &str = if errcode >= 0 && (errcode as usize) < ERROR_MESSAGES.len() {
        ERROR_MESSAGES[errcode as usize]
    } else {
        UNKNOWN_ERROR
    };

    // 完整消息所需的字节数 = 消息长度 + 1（null 终止符）
    let needed = msg.len() + 1;

    // 2. 如果缓冲区非空且有空间，写入消息
    if !errbuf.is_null() && errbuf_size > 0 {
        let msg_bytes = msg.as_bytes();
        // 计算可复制的字节数（不含终止符，为 null 留空间）
        let copy_len = core::cmp::min(msg_bytes.len(), errbuf_size - 1);

        unsafe {
            // 复制消息内容
            core::ptr::copy_nonoverlapping(
                msg_bytes.as_ptr(),
                errbuf as *mut u8,
                copy_len,
            );
            // 添加 null 终止符
            *errbuf.add(copy_len) = 0;
        }
    }

    needed
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

    use super::*;
    use super::super::regcomp::*;

    // ---- 错误消息表测试 ----

    test!("test_error_messages_count" {
        // 应有 14 条消息对应 REG_OK(0) 到 REG_BADRPT(13)
        assert_eq!(ERROR_MESSAGES.len(), 14);
    });

    test!("test_error_messages_index_0" {
        assert_eq!(ERROR_MESSAGES[0], "No error");
    });

    test!("test_error_messages_index_1" {
        assert_eq!(ERROR_MESSAGES[1], "No match");
    });

    test!("test_error_messages_index_2" {
        assert_eq!(ERROR_MESSAGES[2], "Invalid regexp");
    });

    test!("test_error_messages_index_7" {
        assert_eq!(ERROR_MESSAGES[7], "Missing ']'");
    });

    test!("test_error_messages_index_8" {
        assert_eq!(ERROR_MESSAGES[8], "Missing ')'");
    });

    test!("test_error_messages_index_12" {
        assert_eq!(ERROR_MESSAGES[12], "Out of memory");
    });

    test!("test_error_messages_index_13" {
        assert_eq!(ERROR_MESSAGES[13], "Repetition not preceded by valid expression");
    });

    test!("test_error_messages_indices_match_reg_constants" {
        // 验证错误信息索引与 REG_* 常量一致
        assert_eq!(REG_OK as usize, 0);
        assert_eq!(REG_NOMATCH as usize, 1);
        assert_eq!(REG_BADPAT as usize, 2);
        assert_eq!(REG_ECOLLATE as usize, 3);
        assert_eq!(REG_ECTYPE as usize, 4);
        assert_eq!(REG_EESCAPE as usize, 5);
        assert_eq!(REG_ESUBREG as usize, 6);
        assert_eq!(REG_EBRACK as usize, 7);
        assert_eq!(REG_EPAREN as usize, 8);
        assert_eq!(REG_EBRACE as usize, 9);
        assert_eq!(REG_BADBR as usize, 10);
        assert_eq!(REG_ERANGE as usize, 11);
        assert_eq!(REG_ESPACE as usize, 12);
        assert_eq!(REG_BADRPT as usize, 13);
    });

    test!("test_unknown_error_message" {
        assert_eq!(UNKNOWN_ERROR, "Unknown error");
    });

    test!("test_all_error_messages_non_empty" {
        for (i, msg) in ERROR_MESSAGES.iter().enumerate() {
            assert!(!msg.is_empty(), "错误消息 {} 为空", i);
        }
    });

    // ---- regerror 公开 API 测试 ----

    test!("test_regerror_ok" {
        unsafe {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_OK,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
            // 验证缓冲区包含 "No error"
            let s = core::ffi::CStr::from_ptr(buf.as_ptr() as *const c_char);
            let msg = s.to_str().unwrap();
            assert!(msg.contains("error") || msg.contains("No error"));
        }
    });

    test!("test_regerror_nomatch" {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_NOMATCH,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
    });

    test!("test_regerror_badpat" {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_BADPAT,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
    });

    test!("test_regerror_espace" {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_ESPACE,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
    });

    test!("test_regerror_invalid_errcode" {
            let mut buf = [0u8; 256];
            let len = regerror(
                999, // 越界错误码
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            // 应返回 "Unknown error" 的长度
            assert!(len > 0);
    });

    test!("test_regerror_negative_errcode" {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_ENOSYS, // -1
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
    });

    test!("test_regerror_null_buffer" {
            let len = regerror(
                REG_BADPAT,
                core::ptr::null(),
                core::ptr::null_mut(),
                0,
            );
            // 即使缓冲区为 null 且大小为 0，也应返回所需长度
            assert!(len > 0);
    });

    test!("test_regerror_zero_size" {
            let mut buf = [0u8; 256];
            let len = regerror(
                REG_EBRACK,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                0, // size 为 0
            );
            assert!(len > 0);
            // 缓冲区不应被写入
            assert_eq!(buf[0], 0);
    });

    test!("test_regerror_small_buffer" {
            let mut buf = [0u8; 4]; // 仅 4 字节
            let len = regerror(
                REG_BADPAT,
                core::ptr::null(),
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
            );
            assert!(len > 0);
            // 消息被截断，但缓冲区应以 null 结尾
            // 实现后：验证 buf[3] == 0 (或 buf 的最后一字节为 null)
    });

    test!("test_regerror_all_valid_errcodes" {
            let mut buf = [0u8; 256];
            for errcode in 0..=13 {
                let len = regerror(
                    errcode,
                    core::ptr::null(),
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len(),
                );
                assert!(len > 0, "errcode={} 返回了 len=0", errcode);
            }
    });

    test!("test_regerror_preg_ignored" {
        // musl 实现中 preg 参数被完全忽略
        // 传入各种指针值（包括 null）都应正常工作
            let mut buf = [0u8; 256];
            // 传入 null
            let len1 = regerror(REG_OK, core::ptr::null(), buf.as_mut_ptr() as *mut c_char, buf.len());
            assert!(len1 > 0);
    });
}
