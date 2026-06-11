//! printf / fprintf / sprintf / vprintf / vsprintf 集成测试
//!
//! printf / fprintf / sprintf 是可变参数包装函数, 内部委托给
//! vfprintf / vsnprintf 格式化引擎。vprintf / vsprintf 是
//! va_list 版本的格式化函数。
//!
//! 注意: printf / fprintf / sprintf 是直接的 extern "C" varargs 声明,
//! 在 Rust 中调用必须使用 unsafe 块。

use core::ffi::c_char;
use super::imports::{
    fopen, fclose, fflush,
    printf, fprintf, sprintf,
    vsprintf, vprintf,
};
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

// ---- sprintf 测试（向缓冲区写入） ----

test!("sprintf_basic" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"hello\0"),
        )
    };
    assert_eq!(ret, 5);
    assert_eq!(&buf[..5], b"hello");
    assert_eq!(buf[5], 0);
});

test!("sprintf_int" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%d\0"),
            42i32,
        )
    };
    assert_eq!(ret, 2);
    assert_eq!(&buf[..2], b"42");
});

test!("sprintf_zero" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%d\0"),
            0i32,
        )
    };
    assert_eq!(ret, 1);
    assert_eq!(&buf[..1], b"0");
});

test!("sprintf_negative" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%d\0"),
            -123i32,
        )
    };
    assert_eq!(ret, 4);
    assert_eq!(&buf[..4], b"-123");
});

test!("sprintf_string" {
    let mut buf: [u8; 64] = [0; 64];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%s %s!\0"),
            cstr(b"Hello\0"),
            cstr(b"World\0"),
        )
    };
    assert_eq!(ret, 12);
    assert_eq!(&buf[..12], b"Hello World!");
});

test!("sprintf_hex" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%x %X\0"),
            255u32,
            255u32,
        )
    };
    assert_eq!(ret, 5);
    assert_eq!(&buf[..5], b"ff FF");
});

test!("sprintf_char" {
    let mut buf: [u8; 8] = [0; 8];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"%c\0"),
            b'Q' as i32,
        )
    };
    assert_eq!(ret, 1);
    assert_eq!(buf[0], b'Q');
});

test!("sprintf_null_string" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = unsafe {
        sprintf(
            buf.as_mut_ptr() as *mut c_char,
            cstr(b"[%s]\0"),
            core::ptr::null::<c_char>(),
        )
    };
    assert_eq!(ret, 8);
    assert_eq!(&buf[..8], b"[(null)]");
});

// ---- printf 测试（向 stdout 输出） ----

test!("printf_basic" {
    let ret = unsafe { printf(cstr(b"test %d\n\0"), 42i32) };
    assert!(ret >= 0, "printf 应返回 >= 0");
    fflush(core::ptr::null_mut());
});

test!("printf_empty" {
    let ret = unsafe { printf(cstr(b"\0")) };
    assert_eq!(ret, 0);
});

// ---- fprintf 测试（向 FILE 流输出） ----

test!("fprintf_to_dev_null" {
    let path = b"/dev/null\0";
    let f = fopen(cstr(path), cstr(b"w\0"));
    assert!(!f.is_null());
    let ret = unsafe { fprintf(f, cstr(b"value=%d\0"), 99i32) };
    assert_eq!(ret, 8); // "value=99" = 8 chars
    fclose(f);
});

test!("fprintf_null_file" {
    let ret = unsafe { fprintf(core::ptr::null_mut(), cstr(b"test\0")) };
    assert_eq!(ret, -1);
});

// ---- vsprintf 测试（va_list 版本） ----

test!("vsprintf_basic" {
    let mut buf: [u8; 32] = [0; 32];
    let ret = vsprintf(
        buf.as_mut_ptr() as *mut c_char,
        cstr(b"abc\0"),
        core::ptr::null_mut(),
    );
    assert_eq!(ret, 3);
    assert_eq!(&buf[..3], b"abc");
});

// ---- vprintf 测试（va_list 版本，输出到 stdout） ----

test!("vprintf_basic" {
    let _ret = vprintf(
        cstr(b"vprintf-test\n\0"),
        core::ptr::null_mut(),
    );
    // 零参数格式字符串时，vprintf 应正常输出
});
