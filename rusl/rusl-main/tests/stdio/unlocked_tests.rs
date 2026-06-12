//! 免锁 I/O 集成测试
//!
//! fread_unlocked / fwrite_unlocked / fgets_unlocked / fputs_unlocked
//! fflush_unlocked / fgetc_unlocked / getc_unlocked / getchar_unlocked
//! fputc_unlocked / putc_unlocked / putchar_unlocked
//! feof_unlocked / ferror_unlocked / clearerr_unlocked / fileno_unlocked
//!
//! 这些函数操作流但不获取锁。在单线程测试中行为与对应加锁版本一致。
//! musl 中 unlocked 变体是加锁函数的 weak alias, 均不检查 NULL FILE*。

use core::ffi::{c_char, c_int};
use super::imports::{
    fopen, fclose, fflush,
    fread_unlocked, fwrite_unlocked,
    fgets_unlocked, fputs_unlocked,
    fflush_unlocked,
    fgetc_unlocked, getc_unlocked,
    fputc_unlocked, putc_unlocked, putchar_unlocked,
    feof_unlocked, ferror_unlocked, clearerr_unlocked,
    fileno_unlocked,
    stdin, stdout, stderr,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- fread_unlocked ----

test!("fread_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 4] = [0xA5; 4];
    let ret = fread_unlocked(buf.as_mut_ptr() as *mut core::ffi::c_void, 1, 4, f);
    assert_eq!(ret, 0, "/dev/null 应无数据可读");
    assert_eq!(buf[0], 0xA5); // 缓冲区不应被修改
    fclose(f);
});

// ---- fwrite_unlocked ----

test!("fwrite_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let data: [u8; 8] = *b"testdata";
    let ret = fwrite_unlocked(data.as_ptr() as *const core::ffi::c_void, 1, 8, f);
    assert_eq!(ret, 8, "fwrite_unlocked 应写入全部数据");
    fclose(f);
});

// ---- fgets_unlocked ----

test!("fgets_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let mut buf: [u8; 16] = [0; 16];
    let ret = fgets_unlocked(buf.as_mut_ptr() as *mut c_char, 16, f);
    assert!(ret.is_null(), "/dev/null 的 fgets 应返回 NULL");
    fclose(f);
});

// ---- fputs_unlocked ----

test!("fputs_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputs_unlocked(cstr(b"hello\0"), f);
    assert!(ret >= 0, "fputs_unlocked 应返回 >= 0");
    fclose(f);
});

// ---- fflush_unlocked ----

test!("fflush_unlocked_null" {
    let ret = fflush_unlocked(core::ptr::null_mut());
    assert_eq!(ret, 0, "fflush_unlocked(NULL) 应返回 0");
});

test!("fflush_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fflush_unlocked(f);
    assert_eq!(ret, 0, "fflush_unlocked 应返回 0");
    fclose(f);
});

// ---- fgetc_unlocked ----

test!("fgetc_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = fgetc_unlocked(f);
    assert_eq!(ret, -1, "/dev/null 的 fgetc_unlocked 应返回 EOF");
    fclose(f);
});

// ---- getc_unlocked ----

test!("getc_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = getc_unlocked(f);
    assert_eq!(ret, -1, "/dev/null 的 getc_unlocked 应返回 EOF");
    fclose(f);
});

// ---- getchar_unlocked ----

// getchar_unlocked 从 stdin 读取, 无输入时阻塞, 跳过

// ---- fputc_unlocked ----

test!("fputc_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = fputc_unlocked(b'A' as c_int, f);
    assert_eq!(ret, b'A' as c_int, "fputc_unlocked 应返回写入的字符");
    fclose(f);
});

// ---- putc_unlocked ----

test!("putc_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = putc_unlocked(b'X' as c_int, f);
    assert_eq!(ret, b'X' as c_int, "putc_unlocked 应返回写入的字符");
    fclose(f);
});

// ---- putchar_unlocked ----

test!("putchar_unlocked_smoke" {
    let ret = putchar_unlocked(b'?' as c_int);
    if ret != -1 {
        assert_eq!(ret, b'?' as c_int);
    }
    fflush(core::ptr::null_mut());
});

// ---- feof_unlocked ----

test!("feof_unlocked_stdin" {
    // stdin 可能已被之前的测试设置了 EOF, 只验证不崩溃
    let _ = feof_unlocked(unsafe { stdin });
});

test!("feof_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = feof_unlocked(f);
    assert_eq!(ret, 0, "刚打开时 feof_unlocked 应为 0");
    fclose(f);
});

// ---- ferror_unlocked ----

test!("ferror_unlocked_stdin" {
    let ret = ferror_unlocked(unsafe { stdin });
    assert_eq!(ret, 0, "stdin 初始 ferror_unlocked 应为 0");
});

test!("ferror_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let ret = ferror_unlocked(f);
    assert_eq!(ret, 0, "刚打开时 ferror_unlocked 应为 0");
    fclose(f);
});

// ---- clearerr_unlocked ----

test!("clearerr_unlocked_stdin" {
    clearerr_unlocked(unsafe { stdin });
});

// ---- fileno_unlocked ----

test!("fileno_unlocked_stdin" {
    let ret = fileno_unlocked(unsafe { stdin });
    assert_eq!(ret, 0, "stdin fd 应为 0");
});

test!("fileno_unlocked_stdout" {
    let ret = fileno_unlocked(unsafe { stdout });
    assert_eq!(ret, 1, "stdout fd 应为 1");
});

test!("fileno_unlocked_stderr" {
    let ret = fileno_unlocked(unsafe { stderr });
    assert_eq!(ret, 2, "stderr fd 应为 2");
});

test!("fileno_unlocked_dev_null" {
    let f = fopen(cstr(b"/dev/null\0"), cstr(b"r\0"));
    assert!(!f.is_null());
    let fd = fileno_unlocked(f);
    assert!(fd >= 0, "fileno_unlocked 应返回 >= 0");
    fclose(f);
});
